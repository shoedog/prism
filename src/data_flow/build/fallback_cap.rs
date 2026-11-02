//! Count the admitted legacy pairs without constructing FlowEdges.
use crate::data_flow::FlowEdge;

pub(super) const FALLBACK_EDGE_CAP: usize = 1_000_000;

pub(super) struct Emission<'a> {
    edges: &'a mut Vec<FlowEdge>,
    counting: bool,
    count: usize,
}

impl<'a> Emission<'a> {
    pub(super) fn new(edges: &'a mut Vec<FlowEdge>, counting: bool) -> Self {
        Self {
            edges,
            counting,
            count: 0,
        }
    }

    pub(super) fn push(&mut self, edge: impl FnOnce() -> FlowEdge) {
        self.count = self
            .count
            .checked_add(1)
            .expect("DFG prospective edge count overflow");
        if !self.counting {
            self.edges.push(edge());
        }
    }

    pub(super) fn count(&self) -> usize {
        self.count
    }
}
