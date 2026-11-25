//! Candidate continuations used to attribute a sink to a recorded crossing.

use std::collections::{BTreeMap, BTreeSet};

use petgraph::graph::NodeIndex;

use super::{
    BoundaryEdge, BoundaryKind, CodePropertyGraph, CpgNode, LossReason, OrderingWarning, Relation,
    SameLineOrderView, Trace,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ContinuationConstraints {
    pub depth: usize,
    pub relaxations: u32,
    pub recursive: Option<NodeIndex>,
    pub frames: ContinuationFrames,
    pub return_cuts: Vec<(NodeIndex, NodeIndex)>,
}

impl ContinuationConstraints {
    pub(crate) fn new(edge: &BoundaryEdge, trace: &Trace, frames: ContinuationFrames) -> Self {
        let return_cuts = trace
            .cuts
            .iter()
            .filter(|cut| cut.root == edge.root && cut.reason == LossReason::ReturnNotEnteredSite)
            .map(|cut| (cut.from, cut.to))
            .collect();
        Self {
            depth: edge.depth,
            relaxations: relaxation(edge.reason),
            recursive: (edge.kind != BoundaryKind::CrossFunction).then_some(edge.root),
            frames,
            return_cuts,
        }
    }
}

/// Only attribution uses this stack. The live walk retains its legacy state until I-walk.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ContinuationFrames(Vec<Frame>);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Frame {
    site: Option<(String, usize, usize)>,
    to: NodeIndex,
}

impl ContinuationFrames {
    pub(crate) fn advance(
        &mut self,
        cpg: &CodePropertyGraph,
        from: NodeIndex,
        to: NodeIndex,
        relation: Relation,
        order: Option<&dyn SameLineOrderView>,
    ) -> Result<(), LossReason> {
        match relation {
            Relation::CallDescent => {
                let site = crossing_identity(cpg, from, to, relation, order);
                // A repeated site would open a recursive cycle in the attribution search.
                // It supplies no new site certificate for this continuation.
                let frame = Frame { site, to };
                if self.0.contains(&frame) {
                    return Err(LossReason::SelfParam);
                }
                self.0.push(frame);
            }
            Relation::ReturnFlow => {
                let receiving = crossing_identity(cpg, from, to, relation, order);
                Self::check_return(self.0.pop().and_then(|frame| frame.site), receiving)?;
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn check_return(
        entered: Option<(String, usize, usize)>,
        receiving: Option<(String, usize, usize)>,
    ) -> Result<(), LossReason> {
        match (entered, receiving) {
            (Some(entered), Some(receiving)) if entered != receiving => {
                Err(LossReason::ReturnNotEnteredSite)
            }
            _ => Ok(()),
        }
    }
}

fn crossing_identity(
    cpg: &CodePropertyGraph,
    from: NodeIndex,
    to: NodeIndex,
    relation: Relation,
    order: Option<&dyn SameLineOrderView>,
) -> Option<(String, usize, usize)> {
    let site = cpg.crossing_site(from, to, relation, order)?;
    let caller = if relation == Relation::ReturnFlow {
        to
    } else {
        from
    };
    Some((
        cpg.node(caller).file().to_owned(),
        site.start_byte,
        site.end_byte,
    ))
}

pub(crate) fn relaxation(reason: LossReason) -> u32 {
    1 << (reason as u32)
}

impl CodePropertyGraph {
    /// A recorded recursive edge cannot bind an earlier read outside the root's CFG scope.
    pub(crate) fn recursive_boundary_allows(&self, root: NodeIndex, sink: NodeIndex) -> bool {
        if self.node_file_fn(root) != self.node_file_fn(sink) {
            return true;
        }
        // A returned value or parameter is often on a line without a Statement. Only a mapped CFG
        // scope can disprove a continuation, and the walk's continuation semantics also apply here.
        let root = self.node(root);
        let sink_node = self.node(sink);
        if sink_node.line() >= root.line()
            || sink_node.line() == self.node_file_fn(sink).map_or(0, |id| id.2)
        {
            return true;
        }
        self.local_continuation_allows(root.file(), root.line(), sink)
    }

    fn local_continuation_allows(&self, file: &str, line: usize, to: NodeIndex) -> bool {
        if !self.has_cfg_edges() || self.statement_at(file, line).is_none() {
            return true;
        }
        let scope = self.cfg_reachable_lines_unioned(file, line);
        self.cfg_valid(file, line, Some(&scope), true, to)
    }

    /// Search from the precise target node. The DFG supplies candidates, with cuts and missing
    /// occurrence/order evidence handled at the edge where they apply. The queue orders paths by
    /// the number of distinct relaxations and then hop count.
    pub(crate) fn boundary_continuation(
        &self,
        target: NodeIndex,
        constraints: &ContinuationConstraints,
        trace: &Trace,
        order: Option<&dyn SameLineOrderView>,
        ordering_warnings: &mut Vec<OrderingWarning>,
    ) -> BTreeMap<NodeIndex, (u32, usize)> {
        let max_depth = super::trace::MAX_TAINT_DESCENT_DEPTH;
        let initial_depth = (constraints.depth + 1).min(max_depth);
        let mut queue = BTreeSet::from([(
            constraints.relaxations.count_ones(),
            0,
            target,
            initial_depth,
            constraints.relaxations,
            constraints.frames.clone(),
        )]);
        let mut paths = BTreeMap::new();
        let mut visited =
            BTreeMap::<(NodeIndex, usize, ContinuationFrames), Vec<(u32, usize)>>::new();
        let mut gate_cache = BTreeMap::new();
        while let Some((_, length, from, depth, reasons, frames)) = queue.pop_first() {
            let alternatives = visited.entry((from, depth, frames.clone())).or_default();
            if alternatives
                .iter()
                .any(|&(seen, hops)| seen & reasons == seen && hops <= length)
            {
                continue;
            }
            alternatives.retain(|&(seen, hops)| reasons & seen != reasons || length > hops);
            alternatives.push((reasons, length));
            paths.entry(from).or_insert((reasons, length));
            for (to, relation, label, cut) in self.taint_neighbors(from, trace.return_flow_mode) {
                if cut.is_some()
                    || trace.excluded_hops.contains(&(from, to))
                    || (relation == Relation::ReturnFlow
                        && constraints.return_cuts.contains(&(from, to)))
                    || constraints
                        .recursive
                        .is_some_and(|root| !self.recursive_boundary_allows(root, to))
                    || !self.ordering_admits_collect(from, to, relation, order, ordering_warnings)
                {
                    continue;
                }
                let mut required = reasons;
                let mut next_depth = depth;
                let mut next_frames = frames.clone();
                if let Some(kind) = self.crossing_kind(from, to, relation, label) {
                    let crossing = if relation == Relation::ReturnFlow {
                        relation
                    } else {
                        Relation::CallDescent
                    };
                    if next_frames
                        .advance(self, from, to, crossing, order)
                        .is_err()
                    {
                        continue;
                    }
                    if kind == BoundaryKind::CrossFunction && depth == max_depth {
                        required |= relaxation(LossReason::DepthCap);
                    }
                    // The continuation can cross an independently refused hop. Keep its relaxation
                    // in the path score so a route with fewer conditions wins.
                    if let Err(reason) = self.crossing_gate(
                        kind,
                        relation,
                        from,
                        to,
                        label,
                        0,
                        &mut gate_cache,
                        None,
                    ) {
                        required |= relaxation(reason);
                    }
                    next_depth = (depth + 1).min(max_depth);
                } else {
                    let origin = self.node(from);
                    let destination = self.node(to);
                    if matches!(destination, CpgNode::Variable { .. }) {
                        if destination.line() < origin.line()
                            && !self.local_continuation_allows(origin.file(), origin.line(), to)
                        {
                            continue;
                        }
                        // Assignment propagation runs from RHS use to LHS def. In contrast, a
                        // same-line def-use edge cannot certify a read before its definition.
                        if matches!(relation, Relation::DataFlow | Relation::RecoveredDefUse)
                            && origin.line() == destination.line()
                            && matches!(
                                origin,
                                CpgNode::Variable {
                                    access: super::VarAccess::Def,
                                    ..
                                }
                            )
                            && super::trace_cuts::byte_span(destination).0
                                < super::trace_cuts::byte_span(origin).0
                        {
                            continue;
                        }
                    }
                }
                queue.insert((
                    required.count_ones(),
                    length + 1,
                    to,
                    next_depth,
                    required,
                    next_frames,
                ));
            }
        }
        paths
    }
}
