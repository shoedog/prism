//! Immutable exact rows and a separate legacy identity rank.
use super::{AccessPath, VarAccessKind, VarLocation};
use std::cmp::Ordering;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub(in crate::data_flow) struct OccId(pub(in crate::data_flow) u32);
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub(in crate::data_flow) struct LegacyId(pub(in crate::data_flow) u32);

pub(in crate::data_flow) fn checked_id(n: usize) -> Result<u32, &'static str> {
    u32::try_from(n).map_err(|_| "DFG arena exceeds u32 ID space")
}
pub(in crate::data_flow) fn exact_key(
    v: &VarLocation,
) -> (
    &str,
    &str,
    usize,
    usize,
    &AccessPath,
    usize,
    usize,
    VarAccessKind,
) {
    (
        &v.file,
        &v.function,
        v.function_start_line,
        v.line,
        &v.path,
        v.start_byte,
        v.end_byte,
        v.kind,
    )
}
pub(in crate::data_flow) fn exact_cmp(a: &VarLocation, b: &VarLocation) -> Ordering {
    exact_key(a).cmp(&exact_key(b))
}
#[derive(Debug, Clone)]
pub(in crate::data_flow) struct Arena {
    rows: Vec<VarLocation>,
    legacy_reps: Vec<OccId>,
    legacy_by_occ: Vec<LegacyId>,
}
impl Arena {
    pub(in crate::data_flow) fn rows(&self) -> &Vec<VarLocation> {
        &self.rows
    }
    pub(in crate::data_flow) fn legacy_reps(&self) -> &Vec<OccId> {
        &self.legacy_reps
    }
    pub(in crate::data_flow) fn legacy_by_occ(&self) -> &Vec<LegacyId> {
        &self.legacy_by_occ
    }
    pub(super) fn into_wire_parts(self) -> (Vec<VarLocation>, Vec<OccId>, Vec<LegacyId>) {
        (self.rows, self.legacy_reps, self.legacy_by_occ)
    }
    pub(super) fn shrink_to_fit(&mut self) {
        self.rows.shrink_to_fit();
        self.legacy_reps.shrink_to_fit();
        self.legacy_by_occ.shrink_to_fit();
        for row in &mut self.rows {
            row.path.fields.shrink_to_fit();
        }
    }

    pub(in crate::data_flow) fn from_sorted(rows: Vec<VarLocation>) -> Result<Self, &'static str> {
        checked_id(rows.len())?;
        if rows
            .windows(2)
            .any(|w| exact_cmp(&w[0], &w[1]) != Ordering::Less)
        {
            return Err("DFG arena is not strictly exact-sorted");
        }
        let mut legacy_reps: Vec<_> = (0..rows.len()).map(|i| OccId(i as u32)).collect();
        legacy_reps.sort_by(|a, b| rows[a.0 as usize].cmp(&rows[b.0 as usize]));
        legacy_reps.dedup_by(|b, a| rows[a.0 as usize] == rows[b.0 as usize]);
        let legacy_by_occ = rows
            .iter()
            .map(|v| {
                let i = legacy_reps
                    .binary_search_by(|r| rows[r.0 as usize].cmp(v))
                    .unwrap();
                LegacyId(i as u32)
            })
            .collect();
        Ok(Self {
            rows,
            legacy_reps,
            legacy_by_occ,
        })
    }
    pub(in crate::data_flow) fn get(&self, id: OccId) -> &VarLocation {
        &self.rows[id.0 as usize]
    }
    #[cfg(any(test, feature = "dfg-oracle"))]
    pub(in crate::data_flow) fn exact(&self, v: &VarLocation) -> Option<OccId> {
        self.rows
            .binary_search_by(|row| exact_cmp(row, v))
            .ok()
            .map(|i| OccId(i as u32))
    }
    pub(in crate::data_flow) fn legacy(&self, v: &VarLocation) -> Option<LegacyId> {
        self.legacy_reps
            .binary_search_by(|r| self.get(*r).cmp(v))
            .ok()
            .map(|i| LegacyId(i as u32))
    }
    pub(in crate::data_flow) fn group(&self, id: OccId) -> LegacyId {
        self.legacy_by_occ[id.0 as usize]
    }
}

impl Arena {
    pub(in crate::data_flow) fn from_wire(
        rows: Vec<VarLocation>,
        legacy_reps: Vec<OccId>,
        legacy_by_occ: Vec<LegacyId>,
    ) -> Result<Self, &'static str> {
        checked_id(rows.len())?;
        if rows
            .windows(2)
            .any(|w| exact_cmp(&w[0], &w[1]) != Ordering::Less)
        {
            return Err("DFG arena is not strictly exact-sorted");
        }
        if legacy_by_occ.len() != rows.len() {
            return Err("DFG rank length");
        }
        if legacy_reps.iter().any(|id| id.0 as usize >= rows.len()) {
            return Err("DFG rank representative ID");
        }
        if legacy_reps
            .windows(2)
            .any(|w| rows[w[0].0 as usize].cmp(&rows[w[1].0 as usize]) != Ordering::Less)
        {
            return Err("DFG legacy rank order");
        }
        for (i, g) in legacy_by_occ.iter().enumerate() {
            let rep = legacy_reps.get(g.0 as usize).ok_or("DFG legacy rank ID")?;
            if rows[i] != rows[rep.0 as usize] {
                return Err("DFG legacy rank identity");
            }
        }
        // Defensive: strict representative order plus the identity checks above
        // already imply this group equality; do not register an equivalent mutant.
        for (g, rep) in legacy_reps.iter().enumerate() {
            if legacy_by_occ[rep.0 as usize] != LegacyId(g as u32) {
                return Err("DFG rank representative group");
            }
        }
        Ok(Self {
            rows,
            legacy_reps,
            legacy_by_occ,
        })
    }
}
