//! #693/#697: the trace's control for JS/TS/TSX loops.
//!
//! A loop body's normal completion used to reach the code after the loop without passing the header,
//! and a `do` condition shared the `do` header's node (`cfg::js_loops`). With `PRISM_RD_CENSUS_TRACE=1`
//! an owner that holds a loop is solved a second time under main's CFG and statement universe
//! (`cfg::js_loops::with_main_loop_cfg`), so a measurement can name every edge whose label the rule moved
//! (`loop-exit`, or `do-cond` when the edge's lines touch an admitted `do`) and the owners whose
//! availability flipped. Nothing here runs with the trace off.
use super::super::{FlowConfidence, FlowDoubt};
use super::arrow_control::Named;
use super::{
    census_pass::Pass, census_trace, reaching_definitions_impl, ArmProvenance, CfgEdge, DefSite,
    FileCfgEdgeIndex, RdOutcome, RdUnavailable,
};
use crate::ast::ParsedFile;
use crate::cfg::js_loops;
use crate::data_flow::{ExactFlowEdge, FlowEdge, VarLocation};
use std::cell::RefCell;
use std::rc::Rc;
use tree_sitter::Node;

pub(super) const LOOP_EXIT: &str = "loop-exit";
pub(super) const DO_COND: &str = "do-cond";

type FileCfg = Rc<Vec<(CfgEdge, ArmProvenance)>>;

std::thread_local! {
    /// The last file's CFG under main's loop rules: one build per file and thread, not per owner.
    static CONTROL_CFG: RefCell<Option<(String, usize, FileCfg)>> = const { RefCell::new(None) };
}

fn control_cfg(parsed: &ParsedFile) -> FileCfg {
    CONTROL_CFG.with(|cell| {
        let mut slot = cell.borrow_mut();
        if let Some((path, len, edges)) = slot.as_ref() {
            if *path == parsed.path && *len == parsed.source.len() {
                return Rc::clone(edges);
            }
        }
        let edges: FileCfg = Rc::new(js_loops::with_main_loop_cfg(|| {
            crate::cfg::build_cfg_edges_with_arms(parsed)
        }));
        *slot = Some((parsed.path.clone(), parsed.source.len(), Rc::clone(&edges)));
        edges
    })
}

fn availability(outcome: &RdOutcome) -> &'static str {
    match outcome {
        RdOutcome::Available(_) => "ok",
        RdOutcome::Unavailable(RdUnavailable::NoCfgEdges) => "no-cfg",
        RdOutcome::Unavailable(RdUnavailable::DefinitionsCapExceeded { .. }) => "def-cap",
        RdOutcome::Unavailable(RdUnavailable::StatementLinesCapExceeded { .. }) => "line-cap",
    }
}

/// The loops of `owner` (never inside a nested callable): whether it holds any, and the line spans of
/// the `do` loops whose condition is admitted as a statement.
fn loops_of(parsed: &ParsedFile, owner: &Node<'_>) -> (bool, Vec<(usize, usize)>) {
    let Some(body) = owner.child_by_field_name("body") else {
        return (false, Vec::new());
    };
    let boundaries = parsed.language.callable_boundary_node_types();
    let (mut any, mut dos) = (false, Vec::new());
    let mut stack = vec![body];
    while let Some(n) = stack.pop() {
        let kind = n.kind();
        if boundaries.contains(&kind) {
            continue;
        }
        if matches!(
            kind,
            "for_statement" | "for_in_statement" | "while_statement" | "do_statement"
        ) {
            any = true;
            if kind == "do_statement" && parsed.do_condition_line(&n).is_some() {
                dos.push((n.start_position().row + 1, n.end_position().row + 1));
            }
        }
        let mut cursor = n.walk();
        stack.extend(n.named_children(&mut cursor));
    }
    (any, dos)
}

/// `rd-census-owner\tloop-exit\t<file>\t<owner>\t<line>\t<control>><head>` once per owner whose
/// availability flipped, and `loop-exit` / `do-cond` for every edge whose label differs between main's
/// loop CFG and the head's. An unavailable side labels every edge `NameOnly(CfgIncomplete)`.
pub(super) fn trace(
    parsed: &ParsedFile,
    owner: &Node<'_>,
    defs: &[DefSite],
    (dfg_edges, exact_edges): (&[FlowEdge], &[FlowEdge]),
    head: &RdOutcome,
    out: &mut impl std::io::Write,
) -> Named {
    let mut named = Named::default();
    if !parsed.language.is_js_family() {
        return named;
    }
    let (has_loop, dos) = loops_of(parsed, owner);
    if !has_loop {
        return named;
    }
    let (lo, hi) = parsed.node_line_range(owner);
    let all = control_cfg(parsed);
    let own: Vec<(CfgEdge, ArmProvenance)> = all
        .iter()
        .filter(|(e, _)| lo <= e.from_line && e.from_line <= hi)
        .cloned()
        .collect();
    let index = FileCfgEdgeIndex::new(&own);
    let control = js_loops::with_main_loop_cfg(|| {
        reaching_definitions_impl(
            parsed,
            owner,
            defs,
            dfg_edges,
            exact_edges,
            &index,
            Pass::LoopControl,
            &mut None,
        )
    });
    if availability(&control) != availability(head) {
        let _ = writeln!(
            out,
            "rd-census-owner\t{LOOP_EXIT}\t{}\t{}\t{}\t{}>{}",
            parsed.path,
            parsed.dfg_owner_name(owner).unwrap_or_default(),
            owner.start_position().row + 1,
            availability(&control),
            availability(head)
        );
    }
    let refused = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
    let label = |outcome: &RdOutcome, key: &(VarLocation, VarLocation)| match outcome {
        RdOutcome::Available(r) => r.labels.get(key).copied(),
        RdOutcome::Unavailable(_) => Some(refused),
    };
    let exact_label = |outcome: &RdOutcome, key: &ExactFlowEdge| match outcome {
        RdOutcome::Available(r) => r.exact_labels.get(key).copied(),
        RdOutcome::Unavailable(_) => Some(refused),
    };
    let mechanism = |edge: &FlowEdge| {
        let (a, b) = (
            edge.from.line.min(edge.to.line),
            edge.from.line.max(edge.to.line),
        );
        if dos.iter().any(|&(s, e)| s <= b && a <= e) {
            DO_COND
        } else {
            LOOP_EXIT
        }
    };
    for edge in dfg_edges {
        let key = (edge.from.clone(), edge.to.clone());
        let (new, old) = (label(head, &key), label(&control, &key));
        if new.is_some() && old.is_some() && new != old && named.labels.insert(key) {
            census_trace::emit(out, mechanism(edge), edge);
        }
    }
    for edge in exact_edges {
        let key = ExactFlowEdge::from_legacy(edge);
        let (new, old) = (exact_label(head, &key), exact_label(&control, &key));
        if new.is_some() && old.is_some() && new != old && named.exact.insert(key) {
            census_trace::emit(out, mechanism(edge), edge);
        }
    }
    named
}
