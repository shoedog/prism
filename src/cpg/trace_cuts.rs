use std::io::{self, Write};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::call_graph::CallSite;
use crate::resolution::ResolutionConfidence;

use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;

use super::{
    BoundaryKind, CodePropertyGraph, CpgEdge, CpgNode, FlowConfidence, LossReason, Relation,
    ReturnFlowMode, SameLineOrderView, VarAccess,
};

/// A def-use crossing recorded without traversing the refused hop.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BoundaryEdge {
    pub root: NodeIndex,
    pub from: NodeIndex,
    pub to: NodeIndex,
    pub kind: BoundaryKind,
    pub reason: LossReason,
    pub relation: Relation,
    /// Resolved site evidence collected when the crossing ledger is enabled.
    pub site: Option<CrossingSite>,
    pub depth: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CrossingSite {
    pub start_byte: usize,
    pub end_byte: usize,
    pub slot: Option<usize>,
}

impl CrossingSite {
    pub(super) fn for_call(
        site: &CallSite,
        node: NodeIndex,
        relation: Relation,
        order: Option<&dyn SameLineOrderView>,
    ) -> Self {
        Self {
            start_byte: site.start_byte,
            end_byte: site.end_byte,
            slot: order.and_then(|order| order.crossing_slot(node, site, relation)),
        }
    }
}

pub(super) struct SiteEvidence {
    pub location: CrossingSite,
    grades: Vec<ResolutionConfidence>,
}

impl SiteEvidence {
    pub(super) fn exact(location: CrossingSite) -> Self {
        Self {
            location,
            grades: vec![ResolutionConfidence::Exact],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TraceCut {
    pub root: NodeIndex,
    pub from: NodeIndex,
    pub to: NodeIndex,
    pub reason: LossReason,
    pub relation: Relation,
    pub site: Option<CrossingSite>,
    pub depth: usize,
}

#[derive(Clone, Copy)]
pub(super) struct CrossingLedger {
    walk: &'static str,
    id: usize,
}

impl CrossingLedger {
    pub(super) fn start(walk: &'static str) -> Option<Self> {
        static NEXT_WALK: AtomicUsize = AtomicUsize::new(0);
        std::env::var_os("PRISM_TAINT_CROSSING_TRACE")
            .is_some_and(|value| value == "1")
            .then(|| Self {
                walk,
                id: NEXT_WALK.fetch_add(1, Ordering::Relaxed),
            })
    }
}

pub(super) fn byte_span(node: &CpgNode) -> (usize, usize) {
    match node {
        CpgNode::Function {
            start_byte,
            end_byte,
            ..
        }
        | CpgNode::Statement {
            start_byte,
            end_byte,
            ..
        }
        | CpgNode::Variable {
            start_byte,
            end_byte,
            ..
        }
        | CpgNode::ReturnValue {
            start_byte,
            end_byte,
            ..
        } => (*start_byte, *end_byte),
    }
}

impl CodePropertyGraph {
    fn trace_identity(&self, index: NodeIndex) -> String {
        let node = self.node(index);
        let (start, end) = byte_span(node);
        let owner = self.node_file_fn(index);
        let occurrence = match node {
            CpgNode::Variable { path, access, .. } => format!(":{path}:{access:?}"),
            _ => String::new(),
        };
        format!(
            "{}:{}:{start}..{end}#{}@{owner:?}{occurrence}",
            node.file(),
            node.line(),
            index.index()
        )
    }
}

impl CodePropertyGraph {
    pub(super) fn site_evidence(
        &self,
        site: &CallSite,
        location: CrossingSite,
    ) -> Option<SiteEvidence> {
        let grades: Vec<_> = self
            .call_graph
            .resolve_call_site_full(site)
            .resolved
            .into_iter()
            .map(|candidate| candidate.confidence)
            .collect();
        (!grades.is_empty()).then_some(SiteEvidence { location, grades })
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn log_trace_attempt(
        &self,
        ledger: CrossingLedger,
        root: NodeIndex,
        from: NodeIndex,
        to: NodeIndex,
        relation: Relation,
        site: Option<&SiteEvidence>,
        label: Option<FlowConfidence>,
        depth: usize,
        reason: Option<LossReason>,
    ) {
        let event = match (reason, relation) {
            (Some(_), _) => "taint-refusal",
            (None, Relation::ReturnInput) => "taint-return-input",
            (None, _) => "taint-crossing",
        };
        let slot = site
            .and_then(|site| site.location.slot)
            .map_or_else(|| "-".into(), |slot| slot.to_string());
        let component = match self.node(to) {
            CpgNode::ReturnValue {
                child_slot,
                return_start_byte,
                return_end_byte,
                ..
            } => format!(" component={child_slot} return={return_start_byte}..{return_end_byte}"),
            _ => String::new(),
        };
        let reason = reason.map_or_else(String::new, |reason| format!(" reason={reason}"));
        let site_grades = site
            .into_iter()
            .flat_map(|site| &site.grades)
            .map(|grade| format!("{grade:?}"))
            .collect::<Vec<_>>()
            .join(",");
        let site = site.map_or_else(
            || "-".into(),
            |site| format!("{}..{}", site.location.start_byte, site.location.end_byte),
        );
        // The diagnostic has no bearing on the walk, including when stderr is closed.
        let _ = writeln!(io::stderr().lock(),
            "{event} walk={} walk_id={} root={:?} from={:?} to={:?} relation={relation:?} site={site} slot={slot} grades=[{site_grades}] edge_grade={label:?} depth={depth}{component}{reason}",
            ledger.walk, ledger.id,
            self.trace_identity(root), self.trace_identity(from), self.trace_identity(to)
        );
    }
}

impl CodePropertyGraph {
    fn shortcut_site(
        &self,
        use_idx: NodeIndex,
        def_idx: NodeIndex,
    ) -> Option<&crate::call_graph::CallSite> {
        let CpgNode::Variable {
            file: caller_file,
            function: caller_name,
            function_start_line: caller_start,
            start_byte: use_start,
            end_byte: use_end,
            ..
        } = &self.graph[use_idx]
        else {
            return None;
        };

        self.graph
            .edges_directed(def_idx, Direction::Incoming)
            .filter(|edge| {
                matches!(
                    edge.weight(),
                    CpgEdge::ReturnFlow {
                        suppress_shortcut: true
                    }
                )
            })
            .find_map(|edge| {
                let Some((callee_file, callee_name, callee_start)) =
                    self.node_file_fn(edge.source())
                else {
                    return None;
                };
                self.call_graph
                    .calls
                    .iter()
                    .filter(|(id, _)| {
                        id.file == *caller_file
                            && id.name == *caller_name
                            && id.start_line == *caller_start
                    })
                    .flat_map(|(_, sites)| sites)
                    .find(|site| {
                        site.start_byte <= *use_start
                            && *use_end <= site.end_byte
                            && super::build::singleton_exact(
                                &self.call_graph.resolve_call_site_full(site),
                            )
                            .is_some_and(|resolved| {
                                resolved.target.file == callee_file
                                    && resolved.target.name == callee_name
                                    && resolved.target.start_line == callee_start
                            })
                    })
            })
    }

    pub(super) fn taint_neighbors(
        &self,
        node: NodeIndex,
        return_flow_mode: ReturnFlowMode,
    ) -> Vec<(
        NodeIndex,
        Relation,
        Option<FlowConfidence>,
        Option<&CallSite>,
    )> {
        let mut out: Vec<_> = self
            .graph
            .edges(node)
            .filter_map(|edge| match edge.weight() {
                CpgEdge::DataFlow(label) => {
                    Some((edge.target(), Relation::DataFlow, Some(*label), None))
                }
                _ => None,
            })
            .collect();
        // General DFG neighbors can cross lines; NodeIndex order is build-deterministic.
        out.sort_by_key(|(target, _, _, _)| target.index());

        if return_flow_mode == ReturnFlowMode::On {
            let mut returned: Vec<_> = self
                .graph
                .edges(node)
                .filter_map(|edge| match edge.weight() {
                    CpgEdge::ReturnInput => {
                        Some((edge.target(), Relation::ReturnInput, None, None))
                    }
                    CpgEdge::ReturnFlow { .. } => {
                        Some((edge.target(), Relation::ReturnFlow, None, None))
                    }
                    _ => None,
                })
                .collect();
            returned.sort_by_key(|(target, relation, _, _)| {
                (
                    match relation {
                        Relation::ReturnInput => 0,
                        Relation::ReturnFlow => 1,
                        _ => 2,
                    },
                    target.index(),
                )
            });
            out.extend(returned);
        }

        match &self.graph[node] {
            // Use → same-line same-function `Def`s (assignment propagation: `x = y`, the use of `y`
            // flows to the def of `x`). Must stay within the SAME function — keying on (file,line)
            // alone leaks across functions (minified one-line-per-file JS/TS degenerates to
            // everything-taints-everything). Statement/byte-range scoping within a function is a
            // tracked follow-up (line-granular over-approximation).
            CpgNode::Variable {
                access: VarAccess::Use,
                file,
                line,
                function,
                function_start_line,
                ..
            } => {
                if let Some(at) = self.location_index.get(&(file.clone(), *line)) {
                    let mut same: Vec<NodeIndex> = at
                        .iter()
                        .copied()
                        .filter(|&o| {
                            matches!(
                                &self.graph[o],
                                CpgNode::Variable {
                                    access: VarAccess::Def,
                                    function: def_fn,
                                    function_start_line: def_start_line,
                                    ..
                                } if def_fn == function && def_start_line == function_start_line
                            )
                        })
                        .collect();
                    same.sort_by_key(|&i| self.node_sort_key(i));
                    out.extend(same.into_iter().map(|t| {
                        let shortcut = (return_flow_mode == ReturnFlowMode::On)
                            .then(|| self.shortcut_site(node, t))
                            .flatten();
                        (t, Relation::AssignmentPropagation, None, shortcut)
                    }));
                }
            }
            // `Def` → same-function SAME-PATH `Use`s, re-supplying def→use edges `data_flow.rs` drops:
            //  - SIMPLE path (`x`): the DFG carries cross-line def-use, but the variable's *same-line*
            //    edge is dropped (param `ref_line == start`; regular `ref_line == def_line`), so a
            //    same-line `y = u; sink(y)` / `q = p; sink(q)` would dead-end. Recover same-line uses.
            //  - FIELD path (`o.data`): the DFG filters a field's refs to lines AFTER the def
            //    (`collect_path_refs`, `line > def_line`), so a loop-carried `def@N → use@M (M<N)` edge
            //    never exists and the loop's earlier use is a false negative. Recover ALL same-path uses
            //    on ANY line; the back-edge-aware `cfg_valid` prunes the CFG-infeasible ones (loop
            //    back-edges keep the loop-carried use). Same composition simple paths get from the DFG.
            // Same-path only (the variable's own def-use chain, not an assignment), so no cross-variable
            // leak; the safe over-approximation is consistent with the line-granular stance.
            CpgNode::Variable {
                access: VarAccess::Def,
                path,
                ..
            } => {
                let uses = if path.fields.is_empty() {
                    self.same_line_same_path_uses(node)
                } else {
                    self.same_function_same_path_uses_any_line(node)
                };
                for u in uses {
                    out.push((u, Relation::RecoveredDefUse, None, None));
                }
            }
            _ => {}
        }
        out
    }
}
