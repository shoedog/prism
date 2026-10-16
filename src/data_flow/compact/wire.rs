//! Native compact wire and a borrowing serializer with identical field order.
use super::*;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(in crate::data_flow) struct WireDfg {
    pub(in crate::data_flow) arena: Vec<VarLocation>,
    pub(in crate::data_flow) legacy_reps: Vec<OccId>,
    pub(in crate::data_flow) legacy_by_occ: Vec<LegacyId>,
    pub(in crate::data_flow) edges: Vec<(OccId, OccId)>,
    pub(in crate::data_flow) defs: Vec<(BindingKey, Vec<OccId>)>,
    pub(in crate::data_flow) uses: Vec<(BindingKey, Vec<OccId>)>,
    pub(in crate::data_flow) forward: Vec<(LegacyId, Adj)>,
    pub(in crate::data_flow) backward: Vec<(LegacyId, Adj)>,
    pub(in crate::data_flow) labels: Vec<((LegacyId, LegacyId), Label)>,
    pub(in crate::data_flow) exact_labels: Vec<((OccId, OccId), FlowConfidence)>,
    pub(in crate::data_flow) rd_function_stats: BTreeMap<String, crate::cpg::RdFileStats>,
}

use serde::ser::SerializeSeq;
pub(in crate::data_flow) struct MapSeq<'a, K, V>(&'a BTreeMap<K, V>);
impl<K: serde::Serialize, V: serde::Serialize> serde::Serialize for MapSeq<'_, K, V> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(self.0.len()))?;
        for row in self.0 {
            seq.serialize_element(&row)?;
        }
        seq.end()
    }
}
#[derive(serde::Serialize)]
pub(in crate::data_flow) struct WireDfgRef<'a> {
    pub(in crate::data_flow) arena: &'a [VarLocation],
    pub(in crate::data_flow) legacy_reps: &'a [OccId],
    pub(in crate::data_flow) legacy_by_occ: &'a [LegacyId],
    pub(in crate::data_flow) edges: &'a [(OccId, OccId)],
    pub(in crate::data_flow) defs: MapSeq<'a, BindingKey, Vec<OccId>>,
    pub(in crate::data_flow) uses: MapSeq<'a, BindingKey, Vec<OccId>>,
    pub(in crate::data_flow) forward: MapSeq<'a, LegacyId, Adj>,
    pub(in crate::data_flow) backward: MapSeq<'a, LegacyId, Adj>,
    pub(in crate::data_flow) labels: MapSeq<'a, (LegacyId, LegacyId), Label>,
    pub(in crate::data_flow) exact_labels: MapSeq<'a, (OccId, OccId), FlowConfidence>,
    pub(in crate::data_flow) rd_function_stats: &'a BTreeMap<String, crate::cpg::RdFileStats>,
}
impl<'a> From<&'a CompactDfg> for WireDfgRef<'a> {
    fn from(g: &'a CompactDfg) -> Self {
        Self {
            arena: &g.arena.rows,
            legacy_reps: &g.arena.legacy_reps,
            legacy_by_occ: &g.arena.legacy_by_occ,
            edges: &g.edges,
            defs: MapSeq(&g.defs),
            uses: MapSeq(&g.uses),
            forward: MapSeq(&g.forward),
            backward: MapSeq(&g.backward),
            labels: MapSeq(&g.labels),
            exact_labels: MapSeq(&g.exact_labels),
            rd_function_stats: &g.rd_function_stats,
        }
    }
}
impl From<CompactDfg> for WireDfg {
    fn from(g: CompactDfg) -> Self {
        Self {
            arena: g.arena.rows,
            legacy_reps: g.arena.legacy_reps,
            legacy_by_occ: g.arena.legacy_by_occ,
            edges: g.edges,
            defs: g.defs.into_iter().collect(),
            uses: g.uses.into_iter().collect(),
            forward: g.forward.into_iter().collect(),
            backward: g.backward.into_iter().collect(),
            labels: g.labels.into_iter().collect(),
            exact_labels: g.exact_labels.into_iter().collect(),
            rd_function_stats: g.rd_function_stats,
        }
    }
}

impl Serialize for CompactDfg {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        WireDfgRef::from(self).serialize(s)
    }
}
impl<'de> Deserialize<'de> for CompactDfg {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let wire = WireDfg::deserialize(d)?;
        Self::try_from(wire).map_err(serde::de::Error::custom)
    }
}
