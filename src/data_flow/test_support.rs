//! Byte-bearing observations and compact Parts mutation support for unit tests.
use super::DataFlowGraph;
use serde::Serialize;
use std::collections::BTreeMap;
pub(crate) fn bytes<T: Serialize>(value: &T) -> Vec<u8> {
    bincode::serialize(value).expect("observation serializes")
}
pub(crate) fn assert_bytes<T: Serialize>(name: &str, old: &T, new: &T) {
    assert_eq!(bytes(old), bytes(new), "{name}");
}
use crate::data_flow::{FlowPath, VarLocation};
use crate::finding_confidence::{EvidenceHop, EvidencePath};
use crate::frameworks::SanitizerCategory;
use std::collections::BTreeSet;
fn hop_obs(h: &EvidenceHop) -> Vec<u8> {
    match h {
        EvidenceHop::DataFlow {
            from,
            to,
            confidence,
        } => bytes(&(0u8, from, to, confidence)),
        EvidenceHop::Call { edge, confidence } => bytes(&(
            1u8,
            &edge.caller,
            edge.call_site_line,
            edge.confidence,
            edge.kind,
            confidence,
        )),
    }
}
fn category_obs(c: &SanitizerCategory) -> u8 {
    match c {
        SanitizerCategory::Xss => 0,
        SanitizerCategory::Sqli => 1,
        SanitizerCategory::Ssrf => 2,
        SanitizerCategory::Deserialization => 3,
        SanitizerCategory::OsCommand => 4,
        SanitizerCategory::PathTraversal => 5,
    }
}
pub(crate) fn path_bytes(paths: &[FlowPath]) -> Vec<u8> {
    bytes(
        &paths
            .iter()
            .map(|p| {
                (
                    &p.edges,
                    p.cleansed_for.iter().map(category_obs).collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>(),
    )
}
pub(crate) fn labeled_obs(value: &(BTreeSet<VarLocation>, Vec<EvidenceHop>)) -> Vec<u8> {
    bytes(&(&value.0, value.1.iter().map(hop_obs).collect::<Vec<_>>()))
}
pub(crate) fn evidence_obs(paths: &[Option<EvidencePath>]) -> Vec<u8> {
    bytes(
        &paths
            .iter()
            .map(|p| {
                p.as_ref().map(|p| {
                    (
                        p.hops.iter().map(hop_obs).collect::<Vec<_>>(),
                        p.crossed_unlabeled,
                    )
                })
            })
            .collect::<Vec<_>>(),
    )
}
pub(crate) fn current_storage(g: &DataFlowGraph) -> Vec<u8> {
    crate::cpg::dfg_observation::dfg_storage(g)
}
/// Mutable label Parts for deliberately inconsistent, small crate-unit fixtures.
#[cfg(test)]
pub(crate) struct LabelParts<'a> {
    pub labels: &'a mut BTreeMap<(VarLocation, VarLocation), crate::cpg::FlowConfidence>,
    pub exact_labels: &'a mut BTreeMap<super::ExactFlowEdge, crate::cpg::FlowConfidence>,
}
#[cfg(test)]
pub(crate) fn edit_legacy_parts(g: &mut DataFlowGraph, edit: impl FnOnce(&mut LabelParts<'_>)) {
    let mut old = parts::snapshot(g);
    edit(&mut LabelParts {
        labels: &mut old.labels,
        exact_labels: &mut old.exact_labels,
    });
    *g = parts::graph(old);
}

/// Owned edge snapshot for existing serialization assertions; preserves order and spans.
#[cfg(test)]
pub(crate) fn edge_snapshot(g: &DataFlowGraph) -> Vec<super::FlowEdge> {
    g.edges().iter().map(|e| e.to_owned()).collect()
}

/// Owned label snapshot for existing map assertions; includes retained key spans.
#[cfg(test)]
pub(crate) fn label_snapshot(
    g: &DataFlowGraph,
) -> BTreeMap<(VarLocation, VarLocation), crate::cpg::FlowConfidence> {
    g.labels()
        .map(|(e, c)| ((e.from.clone(), e.to.clone()), c))
        .collect()
}

#[cfg(test)]
pub(crate) fn exact_label_snapshot(
    g: &DataFlowGraph,
) -> BTreeMap<super::ExactFlowEdge, crate::cpg::FlowConfidence> {
    g.exact_labels().collect()
}

pub(crate) mod compact;
#[cfg(test)]
pub(crate) mod parts;

#[cfg(test)]
pub(crate) fn assert_codec_validation() {
    super::tests::corruption::codec_validation_matrix();
}

#[cfg(test)]
pub(crate) fn corrupt_production_dfg(g: &DataFlowGraph) -> Vec<u8> {
    let mut wire = super::compact::wire::WireDfg::from(g.storage.clone());
    assert!(!wire.edges.is_empty());
    wire.edges[0].0 = super::compact::OccId(u32::MAX);
    bincode::serialize(&wire).unwrap()
}

#[cfg(test)]
pub(crate) fn assert_native_production_decode() {
    super::tests::activation::production_codec_shares_one_location_per_exact_identity();
}
