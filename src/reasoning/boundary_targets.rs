//! Bind a refused hop to a concrete source/sink pair.

use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};

use petgraph::graph::NodeIndex;

use crate::cpg::{
    crossing_trace_enabled, relaxation, trace_byte_span, BoundaryEdge, CodePropertyGraph,
    ContinuationConstraints, ContinuationFrames, ContinuationReturnSites, ContinuationSearch,
    LossReason, OrderingWarning, SameLineOrderView, Trace, EDGE_OWN_NOT_EXACT,
};

use super::types::Reachability;

const MAX_CONTINUATION_PAIRS: usize = 128;

/// Result of attributing a root and sink to a refused crossing.
#[derive(Debug)]
pub enum BoundaryAttribution<'a> {
    None,
    Matched(&'a BoundaryEdge),
    /// The search hit a pair or step limit. `matched` records a match found by the search.
    Incomplete {
        matched: bool,
    },
}

pub fn reachability_for_node_from(
    cpg: &CodePropertyGraph,
    trace: &Trace,
    root: NodeIndex,
    sink: NodeIndex,
) -> Reachability {
    let mut ordering_warnings = Vec::new();
    reachability_for_node_from_ordered(cpg, trace, root, sink, None, &mut ordering_warnings)
}

pub fn reachability_for_node_from_ordered(
    cpg: &CodePropertyGraph,
    trace: &Trace,
    root: NodeIndex,
    sink: NodeIndex,
    order: Option<&dyn SameLineOrderView>,
    ordering_warnings: &mut Vec<OrderingWarning>,
) -> Reachability {
    reachability_for_node_from_ordered_with_attribution(
        cpg,
        trace,
        root,
        sink,
        order,
        ordering_warnings,
    )
    .0
}

/// Classify a single root/sink pair and retain whether boundary attribution was incomplete.
pub fn reachability_for_node_from_ordered_with_attribution<'a>(
    cpg: &'a CodePropertyGraph,
    trace: &'a Trace,
    root: NodeIndex,
    sink: NodeIndex,
    order: Option<&'a dyn SameLineOrderView>,
    ordering_warnings: &mut Vec<OrderingWarning>,
) -> (Reachability, BoundaryAttribution<'a>) {
    let mut targets = BoundaryTargets::new(cpg, trace, order);
    reachability_and_boundary(trace, &mut targets, root, sink, ordering_warnings)
}

pub(crate) fn reachability_and_boundary<'a>(
    trace: &Trace,
    targets: &mut BoundaryTargets<'a>,
    root: NodeIndex,
    sink: NodeIndex,
    ordering_warnings: &mut Vec<OrderingWarning>,
) -> (Reachability, BoundaryAttribution<'a>) {
    if trace
        .frontier_by_root
        .get(&root)
        .is_some_and(|frontier| frontier.contains(&sink))
    {
        return (Reachability::Reached, BoundaryAttribution::None);
    }
    let attribution = targets.find(root, sink, ordering_warnings);
    let matched = match attribution {
        BoundaryAttribution::Matched(_) => true,
        BoundaryAttribution::Incomplete { matched } => {
            targets.legacy_boundary_matches(root, sink, ordering_warnings) || matched
        }
        BoundaryAttribution::None => false,
    };
    let verdict = if matched {
        Reachability::BoundaryExited
    } else {
        Reachability::NotReached
    };
    (verdict, attribution)
}

struct Continuation {
    search: ContinuationSearch,
    warnings: Vec<OrderingWarning>,
}

pub(crate) struct BoundaryTargets<'a> {
    cpg: &'a CodePropertyGraph,
    trace: &'a Trace,
    order: Option<&'a dyn SameLineOrderView>,
    by_root: BTreeMap<NodeIndex, Vec<&'a BoundaryEdge>>,
    continuation: BTreeMap<(NodeIndex, ContinuationConstraints), Continuation>,
    return_sites: OnceCell<ContinuationReturnSites>,
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
            return_sites: OnceCell::new(),
        }
    }

    fn legacy_boundary_matches(
        &self,
        root: NodeIndex,
        sink: NodeIndex,
        warnings: &mut Vec<OrderingWarning>,
    ) -> bool {
        // Main's local boundary test has no continuation budget. Only targets in the
        // sink's function can reach it through that test.
        let owner = self.cpg.node_file_fn(sink);
        self.by_root.get(&root).into_iter().flatten().any(|edge| {
            edge.to == sink
                || (owner.is_some()
                    && self.cpg.node_file_fn(edge.to) == owner
                    && self
                        .cpg
                        .forward_reachable_in_function_ordered(edge.to, self.order, warnings)
                        .contains(&sink))
        })
    }

    #[cfg(test)]
    pub(super) fn continuation_steps(&self) -> impl Iterator<Item = usize> + '_ {
        self.continuation.values().map(|c| c.search.steps)
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
            .min_by_key(|(edge, prefix, _)| {
                rank(
                    edge,
                    *prefix,
                    ContinuationConstraints::initial_relaxations(edge).count_ones(),
                    0,
                )
            })
        {
            return BoundaryAttribution::Matched(edge);
        }

        let log = crossing_trace_enabled();
        let mut pairs = BTreeSet::new();
        let mut best = None;
        let mut incomplete = false;
        for (edge, prefix, frames) in eligible {
            let key = (
                edge.to,
                ContinuationConstraints::new(edge, self.trace, frames),
            );
            let new_pair = pairs.insert(key.clone());
            if new_pair && pairs.len() > MAX_CONTINUATION_PAIRS {
                incomplete = true;
                break;
            }
            let continuation =
                self.continuation
                    .entry(key)
                    .or_insert_with_key(|(target, constraints)| {
                        let mut warnings = Vec::new();
                        let return_sites = self.return_sites.get_or_init(|| {
                            ContinuationReturnSites::new(
                                cpg,
                                self.trace.return_flow_mode,
                                self.order,
                            )
                        });
                        let search = cpg.boundary_continuation(
                            *target,
                            constraints,
                            self.trace,
                            self.order,
                            return_sites,
                            &mut warnings,
                        );
                        Continuation { search, warnings }
                    });
            if new_pair {
                warnings.extend(continuation.warnings.iter().cloned());
            }
            incomplete |= continuation.search.incomplete;
            if let Some(&(reasons, suffix)) = continuation.search.paths.get(&sink) {
                if log {
                    log_candidate(cpg, edge, sink, reasons, prefix + 1 + suffix);
                }
                let score = rank(edge, prefix, reasons.count_ones(), suffix);
                if best.as_ref().is_none_or(|(previous, _)| score < *previous) {
                    best = Some((score, edge));
                }
            }
        }
        if incomplete {
            if log {
                let _ = writeln!(
                    io::stderr().lock(),
                    "taint-attribution root={:?} sink={:?} status=incomplete explained=false",
                    cpg.trace_identity(root),
                    cpg.trace_identity(sink)
                );
            }
            return BoundaryAttribution::Incomplete {
                matched: best.is_some(),
            };
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
    let secondary = reasons & !ContinuationConstraints::primary_relaxation(edge);
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
    .filter(|&reason| {
        secondary & relaxation(reason) != 0
            || (reason == LossReason::NotExact && secondary & EDGE_OWN_NOT_EXACT != 0)
    })
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
