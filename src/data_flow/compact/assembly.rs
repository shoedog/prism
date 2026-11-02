//! Crate-local, borrowing Step 4 view. IDs are valid only for this immutable DFG.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct AssemblyId(u32);
impl AssemblyId {
    pub(crate) fn slot(self) -> usize {
        self.0 as usize
    }
}

pub(crate) struct AssemblyView<'a>(&'a CompactDfg);
impl DataFlowGraph {
    pub(crate) fn assembly_view(&self) -> AssemblyView<'_> {
        AssemblyView(&self.storage)
    }
}
impl AssemblyView<'_> {
    pub(crate) fn len(&self) -> usize {
        self.0.arena.rows().len()
    }
    pub(crate) fn location(&self, id: AssemblyId) -> &VarLocation {
        self.0.arena.get(OccId(id.0))
    }
    pub(crate) fn edges(&self) -> impl Iterator<Item = (AssemblyId, AssemblyId)> + '_ {
        self.0
            .edges
            .iter()
            .map(|&(f, t)| (AssemblyId(f.0), AssemblyId(t.0)))
    }
    pub(crate) fn label(&self, from: AssemblyId, to: AssemblyId) -> Option<FlowConfidence> {
        self.0
            .labels
            .get(&(
                self.0.arena.group(OccId(from.0)),
                self.0.arena.group(OccId(to.0)),
            ))
            .map(|v| v.confidence)
    }
    pub(crate) fn exact_label(&self, from: AssemblyId, to: AssemblyId) -> Option<FlowConfidence> {
        self.0
            .exact_labels
            .get(&(OccId(from.0), OccId(to.0)))
            .copied()
    }
    pub(crate) fn exact_labels(
        &self,
    ) -> impl Iterator<Item = (AssemblyId, AssemblyId, FlowConfidence)> + '_ {
        self.0
            .exact_labels
            .iter()
            .map(|(&(f, t), &c)| (AssemblyId(f.0), AssemblyId(t.0), c))
    }
}

#[cfg(test)]
thread_local! { pub(crate) static LEGACY_LABEL_PROBES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
#[cfg(test)]
pub(crate) fn legacy_label_probe() {
    LEGACY_LABEL_PROBES.with(|c| c.set(c.get() + 1));
}
#[cfg(test)]
impl DataFlowGraph {
    pub(crate) fn reset_assembly_label_probes() {
        LEGACY_LABEL_PROBES.with(|c| c.set(0));
    }
    pub(crate) fn assembly_label_probes() -> usize {
        LEGACY_LABEL_PROBES.with(std::cell::Cell::get)
    }
}
