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
pub struct Locations<'a> {
    pub(crate) iter: Box<dyn Iterator<Item = &'a VarLocation> + 'a>,
}
impl<'a> Locations<'a> {
    pub fn iter(self) -> Box<dyn Iterator<Item = &'a VarLocation> + 'a> {
        self.iter
    }
    pub fn to_vec(self) -> Vec<VarLocation> {
        self.iter.cloned().collect()
    }
}
impl<'a> IntoIterator for Locations<'a> {
    type Item = &'a VarLocation;
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter
    }
}
impl DataFlowGraph {
    pub fn edges(&self) -> Edges<'_> {
        Edges {
            len: self.edges.len(),
            iter: Box::new(self.edges.iter().map(|e| FlowEdgeRef {
                from: &e.from,
                to: &e.to,
            })),
        }
    }
    pub fn defs(&self) -> impl Iterator<Item = (&BindingKey, Locations<'_>)> {
        self.defs.iter().map(|(k, v)| {
            (
                k,
                Locations {
                    iter: Box::new(v.iter()),
                },
            )
        })
    }
    pub fn uses(&self) -> impl Iterator<Item = (&BindingKey, Locations<'_>)> {
        self.uses.iter().map(|(k, v)| {
            (
                k,
                Locations {
                    iter: Box::new(v.iter()),
                },
            )
        })
    }
    pub fn forward(&self) -> impl Iterator<Item = (&VarLocation, Locations<'_>)> {
        self.forward.iter().map(|(k, v)| {
            (
                k,
                Locations {
                    iter: Box::new(v.iter()),
                },
            )
        })
    }
    pub fn backward(&self) -> impl Iterator<Item = (&VarLocation, Locations<'_>)> {
        self.backward.iter().map(|(k, v)| {
            (
                k,
                Locations {
                    iter: Box::new(v.iter()),
                },
            )
        })
    }
    pub fn successors(&self, at: &VarLocation) -> Option<Locations<'_>> {
        self.forward.get(at).map(|v| Locations {
            iter: Box::new(v.iter()),
        })
    }
    pub fn predecessors(&self, at: &VarLocation) -> Option<Locations<'_>> {
        self.backward.get(at).map(|v| Locations {
            iter: Box::new(v.iter()),
        })
    }
    pub fn label(
        &self,
        from: &VarLocation,
        to: &VarLocation,
    ) -> Option<crate::cpg::FlowConfidence> {
        self.labels.get(&(from.clone(), to.clone())).copied()
    }
    pub fn labels(&self) -> impl Iterator<Item = (FlowEdgeRef<'_>, crate::cpg::FlowConfidence)> {
        self.labels
            .iter()
            .map(|((from, to), &label)| (FlowEdgeRef { from, to }, label))
    }
    pub(crate) fn exact_labels(
        &self,
    ) -> impl Iterator<Item = (super::ExactFlowEdge, crate::cpg::FlowConfidence)> + '_ {
        self.exact_labels
            .iter()
            .map(|(edge, &label)| (edge.clone(), label))
    }
    pub fn rd_function_stats(
        &self,
    ) -> &std::collections::BTreeMap<String, crate::cpg::RdFileStats> {
        &self.rd_function_stats
    }
    pub(crate) fn rd_function_stats_mut(
        &mut self,
    ) -> &mut std::collections::BTreeMap<String, crate::cpg::RdFileStats> {
        &mut self.rd_function_stats
    }
}
