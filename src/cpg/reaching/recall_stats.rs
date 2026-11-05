//! Persisted owner identities for the independent Go recall lane.
use super::FallbackFunctionKey;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) enum Status {
    Admitted(usize),
    DefinitionsOrStatementsCap,
    NoCfg,
    AdditionCap(usize),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecallStats(BTreeMap<FallbackFunctionKey, Status>);

impl Serialize for RecallStats {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(&self.0)
    }
}
impl<'de> Deserialize<'de> for RecallStats {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let rows = Vec::<(FallbackFunctionKey, Status)>::deserialize(d)?;
        if rows.windows(2).any(|w| w[0].0 >= w[1].0) {
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
        Ok(Self(rows.into_iter().collect()))
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
