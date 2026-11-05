//! Persisted owner identities for the independent Go recall lane.
use super::FallbackFunctionKey;
use crate::access_path::AccessPath;
use crate::cpg::FlowConfidence;
use crate::data_flow::VarLocation;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

/// Byte identity of one admitted receiver edge inside its file.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) struct ReceiverKey {
    pub from_start: usize,
    pub from_end: usize,
    pub from_path: AccessPath,
    pub to_start: usize,
    pub to_end: usize,
}

/// A receiver recall edge `Def(p.f) -> Use(R)` of `R.M(..)`, recorded with its
/// selector `M` so the repository-wide pass can refuse it when `M` is a
/// func-typed field of the package (a field read, not a receiver). The edge
/// is present in the graph unless `refused` holds its endpoints.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReceiverEdge {
    pub selector: String,
    pub label: FlowConfidence,
    /// The receiver Use exists only as this lane's endpoint.
    pub new_use: bool,
    /// Endpoints of an edge currently removed from the graph.
    pub refused: Option<(VarLocation, VarLocation)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) enum Status {
    Admitted(usize),
    DefinitionsOrStatementsCap,
    NoCfg,
    AdditionCap(usize),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecallStats(
    BTreeMap<FallbackFunctionKey, Status>,
    BTreeMap<ReceiverKey, ReceiverEdge>,
);

impl Serialize for RecallStats {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeTuple;
        struct Rows<'a, K, V>(&'a BTreeMap<K, V>);
        impl<K: Serialize, V: Serialize> Serialize for Rows<'_, K, V> {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.collect_seq(self.0)
            }
        }
        let mut t = s.serialize_tuple(2)?;
        t.serialize_element(&Rows(&self.0))?;
        t.serialize_element(&Rows(&self.1))?;
        t.end()
    }
}
impl<'de> Deserialize<'de> for RecallStats {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let (rows, receivers) = <(
            Vec<(FallbackFunctionKey, Status)>,
            Vec<(ReceiverKey, ReceiverEdge)>,
        )>::deserialize(d)?;
        if rows.windows(2).any(|w| w[0].0 >= w[1].0)
            || receivers.windows(2).any(|w| w[0].0 >= w[1].0)
        {
            return Err(serde::de::Error::custom("DFG recall map order/duplicate"));
        }
        rows.iter()
            .try_fold(0usize, |sum, (_, status)| {
                sum.checked_add(match status {
                    Status::Admitted(n) | Status::AdditionCap(n) => *n,
                    _ => 0,
                })
            })
            .ok_or_else(|| serde::de::Error::custom("DFG recall count overflow"))?;
        Ok(Self(
            rows.into_iter().collect(),
            receivers.into_iter().collect(),
        ))
    }
}
impl RecallStats {
    pub(crate) fn record(&mut self, key: FallbackFunctionKey, status: Status) {
        self.0
            .entry(key)
            .and_modify(|old| *old = (*old).max(status))
            .or_insert(status);
    }
    pub(crate) fn merge(&mut self, other: Self) {
        for (key, status) in other.0 {
            self.record(key, status);
        }
        self.1.extend(other.1);
    }
    pub(crate) fn record_receiver(&mut self, key: ReceiverKey, edge: ReceiverEdge) {
        self.1.entry(key).or_insert(edge);
    }
    pub(crate) fn receivers(&self) -> &BTreeMap<ReceiverKey, ReceiverEdge> {
        &self.1
    }
    pub(crate) fn receivers_mut(&mut self) -> &mut BTreeMap<ReceiverKey, ReceiverEdge> {
        &mut self.1
    }
    /// Receiver edges currently removed because the selector is a func-typed
    /// field of the package.
    pub fn receivers_refused(&self) -> usize {
        self.1.values().filter(|e| e.refused.is_some()).count()
    }
    pub fn functions_over_cap(&self) -> usize {
        self.0
            .values()
            .filter(|s| matches!(s, Status::DefinitionsOrStatementsCap))
            .count()
    }
    pub fn functions_without_cfg(&self) -> usize {
        self.0
            .values()
            .filter(|s| matches!(s, Status::NoCfg))
            .count()
    }
    pub fn functions_capped(&self) -> usize {
        self.0
            .values()
            .filter(|s| matches!(s, Status::AdditionCap(_)))
            .count()
    }
    pub fn candidates_capped(&self) -> usize {
        self.0
            .values()
            .filter_map(|s| match s {
                Status::AdditionCap(n) => Some(n),
                _ => None,
            })
            .sum()
    }
    pub fn candidates(&self) -> usize {
        self.0
            .values()
            .filter_map(|s| match s {
                Status::AdditionCap(n) | Status::Admitted(n) => Some(n),
                _ => None,
            })
            .sum()
    }
    pub fn max_candidates(&self) -> usize {
        self.0
            .values()
            .filter_map(|s| match s {
                Status::AdditionCap(n) | Status::Admitted(n) => Some(*n),
                _ => None,
            })
            .max()
            .unwrap_or(0)
    }
}
