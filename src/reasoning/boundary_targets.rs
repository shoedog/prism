//! Bind a refused hop to a concrete source/sink pair.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};

use petgraph::graph::NodeIndex;

use crate::cpg::{
    crossing_trace_enabled, relaxation, trace_byte_span, BoundaryEdge, CodePropertyGraph,
    ContinuationConstraints, ContinuationFrames, LossReason, OrderingWarning, SameLineOrderView,
    Trace,
};

const MAX_CONTINUATION_PAIRS: usize = 128;

pub(crate) enum BoundaryAttribution<'a> {
    None,
    Matched(&'a BoundaryEdge),
    Incomplete,
}

struct Continuation {
    paths: BTreeMap<NodeIndex, (u32, usize)>,
    warnings: Vec<OrderingWarning>,
}

pub(crate) struct BoundaryTargets<'a> {
    cpg: &'a CodePropertyGraph,
    trace: &'a Trace,
    order: Option<&'a dyn SameLineOrderView>,
    by_root: BTreeMap<NodeIndex, Vec<&'a BoundaryEdge>>,
    continuation: BTreeMap<(NodeIndex, ContinuationConstraints), Continuation>,
}

impl<'a> BoundaryTargets<'a> {
    pub(crate) fn new(
        cpg: &'a CodePropertyGraph,
        trace: &'a Trace,
        order: Option<&'a dyn SameLineOrderView>,
    ) -> Self {
        let mut by_root: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for edge in &trace.boundary {
            by_root.entry(edge.root).or_default().push(edge);
        }
        for edges in by_root.values_mut() {
            edges.sort_by_key(|edge| {
                (
                    node_key(cpg, edge.to),
                    node_key(cpg, edge.from),
                    edge.site,
                    edge.relation,
                )
            });
        }
        Self {
            cpg,
            trace,
            order,
            by_root,
            continuation: BTreeMap::new(),
        }
    }

    pub(crate) fn find(
        &mut self,
        root: NodeIndex,
        sink: NodeIndex,
        warnings: &mut Vec<OrderingWarning>,
    ) -> BoundaryAttribution<'a> {
        let Some(edges) = self.by_root.get(&root) else {
            return BoundaryAttribution::None;
        };
        let cpg = self.cpg;
        let mut prefixes = BTreeMap::new();
        let eligible: Vec<_> = edges
            .iter()
            .copied()
            .filter_map(|edge| {
                if edge.kind != crate::cpg::BoundaryKind::CrossFunction
                    && !cpg.recursive_boundary_allows(root, sink)
                {
                    return None;
                }
                let (length, prefix) = prefixes
                    .entry(edge.from)
                    .or_insert_with(|| prefix(cpg, self.trace, self.order, root, edge.from))
                    .as_ref()?;
                let mut frames = prefix.clone();
                frames
                    .advance(cpg, edge.from, edge.to, edge.relation, self.order)
                    .ok()?;
                Some((edge, *length, frames))
            })
            .collect();
        let rank = |edge: &BoundaryEdge, prefix, relaxations, suffix| {
            let reason = if edge.edge_own_not_exact && edge.reason == LossReason::NotExact {
                (LossReason::DepthCap, 1)
            } else {
                (edge.reason, 0)
            };
            (
                relaxations,
                prefix + 1 + suffix,
                reason,
                node_key(cpg, edge.from),
                node_key(cpg, edge.to),
                edge.site,
                edge.relation,
            )
        };

        if let Some((edge, _, _)) = eligible
            .iter()
            .filter(|(edge, _, _)| edge.to == sink)
            .min_by_key(|(edge, prefix, _)| rank(edge, *prefix, 1, 0))
        {
            return BoundaryAttribution::Matched(edge);
        }

        let log = crossing_trace_enabled();
        let mut pairs = BTreeSet::new();
        let mut best = None;
        for (edge, prefix, frames) in eligible {
            let key = (
                edge.to,
                ContinuationConstraints::new(edge, self.trace, frames),
            );
            let new_pair = pairs.insert(key.clone());
            if new_pair && pairs.len() > MAX_CONTINUATION_PAIRS {
                if log {
                    let _ = writeln!(
                        io::stderr().lock(),
                        "taint-attribution root={:?} sink={:?} status=incomplete explained=false",
                        cpg.trace_identity(root),
                        cpg.trace_identity(sink)
                    );
                }
                return BoundaryAttribution::Incomplete;
            }
            let continuation =
                self.continuation
                    .entry(key)
                    .or_insert_with_key(|(target, constraints)| {
                        let mut warnings = Vec::new();
                        let paths = cpg.boundary_continuation(
                            *target,
                            constraints,
                            self.trace,
                            self.order,
                            &mut warnings,
                        );
                        Continuation { paths, warnings }
                    });
            if new_pair {
                warnings.extend(continuation.warnings.iter().cloned());
            }
            if let Some(&(reasons, suffix)) = continuation.paths.get(&sink) {
                if log {
                    log_candidate(cpg, edge, sink, reasons, prefix + 1 + suffix);
                }
                let score = rank(edge, prefix, reasons.count_ones(), suffix);
                if best.as_ref().is_none_or(|(previous, _)| score < *previous) {
                    best = Some((score, edge));
                }
            }
        }
        best.map_or(BoundaryAttribution::None, |(_, edge)| {
            BoundaryAttribution::Matched(edge)
        })
    }
}

fn log_candidate(
    cpg: &CodePropertyGraph,
    edge: &BoundaryEdge,
    sink: NodeIndex,
    reasons: u32,
    length: usize,
) {
    let joint = [
        LossReason::SelfParam,
        LossReason::SelfReturn,
        LossReason::DepthCap,
        LossReason::TargetNotBinding,
        LossReason::NoMatchingSite,
        LossReason::AmbiguousSite,
        LossReason::NotExact,
        LossReason::UnprovenSlot,
        LossReason::PolicyCeiling,
    ]
    .into_iter()
    .filter(|reason| *reason != edge.reason && reasons & relaxation(*reason) != 0)
    .map(|reason| reason.to_string())
    .collect::<Vec<_>>()
    .join(",");
    let _ = writeln!(io::stderr().lock(),
        "taint-attribution root={:?} sink={:?} from={:?} to={:?} primary={} joint=[{joint}] length={length}",
        cpg.trace_identity(edge.root), cpg.trace_identity(sink), cpg.trace_identity(edge.from),
        cpg.trace_identity(edge.to), edge.reason);
}

fn node_key(cpg: &CodePropertyGraph, index: NodeIndex) -> (&str, usize, usize, NodeIndex) {
    let node = cpg.node(index);
    let (start, end) = trace_byte_span(node);
    (node.file(), start, end, index)
}

fn prefix(
    cpg: &CodePropertyGraph,
    trace: &Trace,
    order: Option<&dyn SameLineOrderView>,
    root: NodeIndex,
    mut node: NodeIndex,
) -> Option<(usize, ContinuationFrames)> {
    let mut chain = Vec::new();
    while node != root {
        let &(parent, relation) = trace.parents_by_root.get(&(root, node))?;
        chain.push((parent, node, relation));
        node = parent;
    }
    let mut frames = ContinuationFrames::default();
    for &(from, to, relation) in chain.iter().rev() {
        frames.advance(cpg, from, to, relation, order).ok()?;
    }
    Some((chain.len(), frames))
}
