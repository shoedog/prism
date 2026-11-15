//! Optional crossing policy for the ordinary forward taint walk (#602 S3a.2).
//!
//! A caller that analyses a graph holding extra "context" files can ask the forward walk to follow
//! an edge that ENTERS a context file from another file only when the edge itself is good enough.
//! The test runs where the walk would queue the edge, so a refused edge never marks its target
//! visited, never becomes its parent and never lends it a label: a longer permitted route to the
//! same target is still found. With no policy the walk is the unchanged legacy traversal, and the
//! policy-aware and policy-free entry points run the same loop.
//!
//! Scope: the labeled BFS, the CFG-constrained adapter, the no-CFG fallback and (#602 S3b-C) the
//! call descent of the return-flow walker (`trace.rs`).

use std::collections::{BTreeMap, BTreeSet};

use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;

use crate::data_flow::{FlowPath, VarLocation};
use crate::finding_confidence::EvidencePath;

use super::{CodePropertyGraph, CpgEdge, FlowConfidence};

/// The weakest data-flow label a context-file entry may carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CrossingCeiling {
    /// A context entry must itself be an `Exact` edge.
    Exact,
    /// A context entry may carry either label; labels are never changed.
    NameOnly,
}

/// Admission policy for edges that enter a context file. There is no `Default`: the caller states
/// the ceiling.
#[derive(Debug, Clone, Copy)]
pub struct TaintCrossingPolicy<'a> {
    context_files: &'a BTreeSet<String>,
    ceiling: CrossingCeiling,
}

impl<'a> TaintCrossingPolicy<'a> {
    pub fn new(context_files: &'a BTreeSet<String>, ceiling: CrossingCeiling) -> Self {
        Self {
            context_files,
            ceiling,
        }
    }

    /// Whether the edge `from_file -> to_file` carrying `edge` may be followed. An edge is refused
    /// exactly when it leaves its file, lands in a context file, the ceiling is `Exact` and THIS
    /// edge's own label is not `Exact` (never the accumulated path label). The files the walk
    /// started from are not an input.
    pub(crate) fn admits(&self, from_file: &str, to_file: &str, edge: FlowConfidence) -> bool {
        let same_file = from_file == to_file;
        let enters_context = self.context_files.contains(to_file);
        let refuses_edge = self.ceiling == CrossingCeiling::Exact && !edge.is_exact();
        same_file || !enters_context || !refuses_edge
    }
}

impl CodePropertyGraph {
    /// The single admission test of the forward walk: no policy admits everything.
    pub(crate) fn crossing_admitted(
        &self,
        policy: Option<&TaintCrossingPolicy<'_>>,
        from: NodeIndex,
        to: NodeIndex,
        edge: FlowConfidence,
    ) -> bool {
        policy.is_none_or(|policy| {
            policy.admits(self.graph[from].file(), self.graph[to].file(), edge)
        })
    }

    /// The worst label among the DataFlow edges `from -> to` that `policy` admits, `None` when no
    /// DataFlow edge exists or none is admitted. The return walker's neighbor list carries no
    /// labels, so a call descent asks this instead: one admitted parallel edge is enough, and the
    /// worst is taken over the admitted edges only, never over all of them (the result does not
    /// depend on edge insertion order).
    pub(crate) fn admitted_data_flow_label(
        &self,
        policy: Option<&TaintCrossingPolicy<'_>>,
        from: NodeIndex,
        to: NodeIndex,
    ) -> Option<FlowConfidence> {
        self.graph
            .edges(from)
            .filter(|edge| edge.target() == to)
            .filter_map(|edge| match edge.weight() {
                CpgEdge::DataFlow(label) => Some(*label),
                _ => None,
            })
            .filter(|&label| self.crossing_admitted(policy, from, to, label))
            .reduce(FlowConfidence::worst)
    }

    /// The legacy forward walk: the policy-aware loop with no policy.
    pub(crate) fn dfg_forward_reachable_with_evidence(
        &self,
        from: &VarLocation,
    ) -> (
        BTreeSet<VarLocation>,
        BTreeMap<NodeIndex, FlowConfidence>,
        BTreeMap<NodeIndex, EvidencePath>,
    ) {
        self.dfg_forward_reachable_with_evidence_policy(from, None)
    }

    /// The legacy line-seeded forward walk: the policy-aware loop with no policy.
    pub(crate) fn taint_forward_labeled(
        &self,
        taint_sources: &[(String, usize)],
    ) -> (
        Vec<FlowPath>,
        BTreeMap<(VarLocation, VarLocation), EvidencePath>,
    ) {
        self.taint_forward_labeled_policy(taint_sources, None)
    }

    /// Forward reachability from `from`, refusing the context-entry edges `policy` does not admit.
    pub fn dfg_forward_reachable_with_policy(
        &self,
        from: &VarLocation,
        policy: &TaintCrossingPolicy<'_>,
    ) -> BTreeSet<VarLocation> {
        self.dfg_forward_reachable_with_evidence_policy(from, Some(policy))
            .0
    }

    /// `taint_forward_cfg_labeled` under a crossing policy (CFG-constrained, or the no-CFG
    /// fallback when the graph has no control-flow edges).
    pub fn taint_forward_cfg_labeled_with_policy(
        &self,
        taint_sources: &[(String, usize)],
        policy: &TaintCrossingPolicy<'_>,
    ) -> (
        Vec<FlowPath>,
        BTreeMap<(VarLocation, VarLocation), EvidencePath>,
    ) {
        self.taint_forward_cfg_labeled_policy(taint_sources, Some(policy))
    }

    /// `taint_forward_cfg_with_return_flow_labeled` under a crossing policy: a call descent into a
    /// context file whose parallel DataFlow edges all fail the ceiling is refused and recorded as
    /// a cross-function boundary.
    pub fn taint_forward_cfg_with_return_flow_labeled_with_policy(
        &self,
        sources: &[(String, usize)],
        policy: &TaintCrossingPolicy<'_>,
    ) -> (
        Vec<FlowPath>,
        BTreeMap<(VarLocation, VarLocation), EvidencePath>,
    ) {
        self.taint_forward_cfg_with_return_flow_labeled_policy(sources, Some(policy))
    }
}
