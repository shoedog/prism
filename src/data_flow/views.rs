//! Ordered borrowed views. Iterator state owns no expanded graph records.
use super::{AccessPath, DataFlowGraph, FlowEdge, VarLocation};

pub type BindingKey = (String, String, usize, AccessPath);
#[derive(Clone, Copy)]
pub struct FlowEdgeRef<'a> {
    pub from: &'a VarLocation,
    pub to: &'a VarLocation,
}
impl FlowEdgeRef<'_> {
    pub fn to_owned(self) -> FlowEdge {
        FlowEdge {
            from: self.from.clone(),
            to: self.to.clone(),
        }
    }
}
pub struct Edges<'a> {
    pub(crate) len: usize,
    pub(crate) iter: Box<dyn Iterator<Item = FlowEdgeRef<'a>> + 'a>,
}
impl<'a> Edges<'a> {
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn iter(self) -> Box<dyn Iterator<Item = FlowEdgeRef<'a>> + 'a> {
        self.iter
    }
}
impl DataFlowGraph {
    pub fn edges(&self) -> Edges<'_> {
        self.storage.edges()
    }
    pub fn defs(&self) -> impl Iterator<Item = (&BindingKey, super::Locations<'_>)> {
        self.storage.defs()
    }
    pub fn uses(&self) -> impl Iterator<Item = (&BindingKey, super::Locations<'_>)> {
        self.storage.uses()
    }
    pub fn forward(&self) -> impl Iterator<Item = (&VarLocation, super::Locations<'_>)> {
        self.storage.forward()
    }
    pub fn backward(&self) -> impl Iterator<Item = (&VarLocation, super::Locations<'_>)> {
        self.storage.backward()
    }
    pub fn successors(&self, at: &VarLocation) -> Option<super::Locations<'_>> {
        self.storage.successors(at)
    }
    pub fn predecessors(&self, at: &VarLocation) -> Option<super::Locations<'_>> {
        self.storage.predecessors(at)
    }
    pub fn label(
        &self,
        from: &VarLocation,
        to: &VarLocation,
    ) -> Option<crate::cpg::FlowConfidence> {
        self.storage.label(from, to)
    }
    pub fn labels(&self) -> impl Iterator<Item = (FlowEdgeRef<'_>, crate::cpg::FlowConfidence)> {
        self.storage.labels()
    }
    pub(crate) fn exact_labels(
        &self,
    ) -> impl Iterator<Item = (super::ExactFlowEdge, crate::cpg::FlowConfidence)> + '_ {
        self.storage.exact_labels()
    }
    pub fn rd_function_stats(
        &self,
    ) -> &std::collections::BTreeMap<String, crate::cpg::RdFileStats> {
        self.storage.rd_function_stats()
    }
    pub(crate) fn rd_function_stats_mut(
        &mut self,
    ) -> &mut std::collections::BTreeMap<String, crate::cpg::RdFileStats> {
        &mut self.storage.rd_function_stats
    }
}
