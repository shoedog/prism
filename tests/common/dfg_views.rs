//! Owned snapshots for map/serde assertions during the accessor migration.
use super::*;
use prism::cpg::FlowConfidence;
use prism::data_flow::VarLocation;

pub fn dfg_labels(g: &DataFlowGraph) -> BTreeMap<(VarLocation, VarLocation), FlowConfidence> {
    g.labels()
        .map(|(edge, label)| ((edge.from.clone(), edge.to.clone()), label))
        .collect()
}
