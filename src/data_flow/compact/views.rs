//! Ordered borrowed compact views; no per-row iterator boxes.
use super::*;
/// Stack-resident ID walk: creating a row view does not allocate.
pub struct IdIter<'a> {
    arena: &'a Arena,
    ids: std::slice::Iter<'a, OccId>,
}
impl<'a> Iterator for IdIter<'a> {
    type Item = &'a VarLocation;
    fn next(&mut self) -> Option<Self::Item> {
        self.ids.next().map(|id| self.arena.get(*id))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.ids.size_hint()
    }
}
impl ExactSizeIterator for IdIter<'_> {}
pub struct Locations<'a> {
    iter: IdIter<'a>,
}
impl<'a> Locations<'a> {
    pub fn iter(self) -> IdIter<'a> {
        self.iter
    }
    pub fn to_vec(self) -> Vec<VarLocation> {
        self.iter.cloned().collect()
    }
}
impl<'a> IntoIterator for Locations<'a> {
    type Item = &'a VarLocation;
    type IntoIter = IdIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter
    }
}
impl CompactDfg {
    pub(in crate::data_flow) fn locations<'a>(&'a self, ids: &'a [OccId]) -> Locations<'a> {
        Locations {
            iter: IdIter {
                arena: &self.arena,
                ids: ids.iter(),
            },
        }
    }
    pub(in crate::data_flow) fn edges(&self) -> Edges<'_> {
        Edges {
            len: self.edges.len(),
            iter: Box::new(self.edges.iter().map(|&(f, t)| FlowEdgeRef {
                from: self.arena.get(f),
                to: self.arena.get(t),
            })),
        }
    }
    pub(in crate::data_flow) fn defs(&self) -> impl Iterator<Item = (&BindingKey, Locations<'_>)> {
        self.defs.iter().map(|(k, v)| (k, self.locations(v)))
    }
    pub(in crate::data_flow) fn uses(&self) -> impl Iterator<Item = (&BindingKey, Locations<'_>)> {
        self.uses.iter().map(|(k, v)| (k, self.locations(v)))
    }
    pub(in crate::data_flow) fn forward(
        &self,
    ) -> impl Iterator<Item = (&VarLocation, Locations<'_>)> {
        self.forward.values().map(|v| {
            (
                self.arena.get(v.representative),
                self.locations(&v.neighbors),
            )
        })
    }
    pub(in crate::data_flow) fn backward(
        &self,
    ) -> impl Iterator<Item = (&VarLocation, Locations<'_>)> {
        self.backward.values().map(|v| {
            (
                self.arena.get(v.representative),
                self.locations(&v.neighbors),
            )
        })
    }
    pub(in crate::data_flow) fn successors(&self, at: &VarLocation) -> Option<Locations<'_>> {
        self.forward
            .get(&self.arena.legacy(at)?)
            .map(|v| self.locations(&v.neighbors))
    }
    pub(in crate::data_flow) fn predecessors(&self, at: &VarLocation) -> Option<Locations<'_>> {
        self.backward
            .get(&self.arena.legacy(at)?)
            .map(|v| self.locations(&v.neighbors))
    }
    pub(in crate::data_flow) fn label(
        &self,
        from: &VarLocation,
        to: &VarLocation,
    ) -> Option<FlowConfidence> {
        self.labels
            .get(&(self.arena.legacy(from)?, self.arena.legacy(to)?))
            .map(|v| v.confidence)
    }
    pub(in crate::data_flow) fn labels(
        &self,
    ) -> impl Iterator<Item = (FlowEdgeRef<'_>, FlowConfidence)> {
        self.labels.values().map(|v| {
            (
                FlowEdgeRef {
                    from: self.arena.get(v.from),
                    to: self.arena.get(v.to),
                },
                v.confidence,
            )
        })
    }
    pub(in crate::data_flow) fn exact_labels(
        &self,
    ) -> impl Iterator<Item = (ExactFlowEdge, FlowConfidence)> + '_ {
        self.exact_labels.iter().map(|(&(f, t), &c)| {
            (
                ExactFlowEdge {
                    from: ExactVarLocation::from_legacy(self.arena.get(f)),
                    to: ExactVarLocation::from_legacy(self.arena.get(t)),
                },
                c,
            )
        })
    }
    pub(in crate::data_flow) fn rd_function_stats(
        &self,
    ) -> &BTreeMap<String, crate::cpg::RdFileStats> {
        &self.rd_function_stats
    }
}
