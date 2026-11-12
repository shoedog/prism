//! Compact production engine with exact occurrence IDs and ordered legacy ranks.
use super::*;
pub(super) mod arena;
pub(super) use arena::*;
pub(crate) mod assembly;
mod freeze;
#[cfg(test)]
pub(super) use freeze::FROM_PARTS_CALLS;
pub(super) mod views;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct Adj {
    pub(super) representative: OccId,
    pub(super) neighbors: Vec<OccId>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct Label {
    pub(super) from: OccId,
    pub(super) to: OccId,
    pub(super) confidence: FlowConfidence,
}
pub(super) fn add_adj(map: &mut BTreeMap<LegacyId, Adj>, a: &Arena, key: OccId, val: OccId) {
    map.entry(a.group(key))
        .or_insert_with(|| Adj {
            representative: key,
            neighbors: Vec::new(),
        })
        .neighbors
        .push(val);
}
pub(super) fn add_label(
    map: &mut BTreeMap<(LegacyId, LegacyId), Label>,
    a: &Arena,
    from: OccId,
    to: OccId,
    confidence: FlowConfidence,
) {
    map.entry((a.group(from), a.group(to)))
        .and_modify(|old| old.confidence = old.confidence.worst(confidence))
        .or_insert(Label {
            from,
            to,
            confidence,
        });
}
type IdEdge = (OccId, OccId);
#[derive(Debug, Clone)]
pub(super) struct CompactDfg {
    pub(super) arena: Arena,
    pub(super) edges: Vec<IdEdge>,
    pub(super) defs: BTreeMap<BindingKey, Vec<OccId>>,
    pub(super) uses: BTreeMap<BindingKey, Vec<OccId>>,
    pub(super) forward: BTreeMap<LegacyId, Adj>,
    pub(super) backward: BTreeMap<LegacyId, Adj>,
    pub(super) labels: BTreeMap<(LegacyId, LegacyId), Label>,
    pub(super) exact_labels: BTreeMap<IdEdge, FlowConfidence>,
    pub(super) rd_function_stats: BTreeMap<String, crate::cpg::RdFileStats>,
    /// Digest of the Rust macro census this graph was built under (empty for none); a
    /// partial cache hit rebuilds in full when a fresh census digests differently.
    pub(super) rust_macro_census_digest: String,
    /// Digest of the C/C++ macro census (#630 slice 6a); same guard, same shape.
    pub(super) c_macro_census_digest: String,
}
#[derive(Clone)]
pub(super) struct Parts {
    pub edges: Vec<FlowEdge>,
    pub defs: BTreeMap<BindingKey, Vec<VarLocation>>,
    pub uses: BTreeMap<BindingKey, Vec<VarLocation>>,
    pub labels: BTreeMap<(VarLocation, VarLocation), FlowConfidence>,
    pub exact_labels: BTreeMap<ExactFlowEdge, FlowConfidence>,
    pub rd_function_stats: BTreeMap<String, crate::cpg::RdFileStats>,
    pub adjacency: Option<(
        BTreeMap<VarLocation, Vec<VarLocation>>,
        BTreeMap<VarLocation, Vec<VarLocation>>,
    )>,
}

mod receiver_refusal;
mod update;
pub(super) use update::combine;

mod validate;
pub(super) mod wire;
