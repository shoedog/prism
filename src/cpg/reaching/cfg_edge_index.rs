//! File-local ordinal index. Admission remains line-based, including overlaps.
use super::{ArmProvenance, CfgEdge, Line};
use std::collections::BTreeMap;

type SelectedEdge = (usize, usize, CfgEdge, ArmProvenance);

pub(crate) struct FileCfgEdgeIndex<'a> {
    edges: &'a [(CfgEdge, ArmProvenance)],
    by_source: BTreeMap<Line, Vec<usize>>,
}

impl<'a> FileCfgEdgeIndex<'a> {
    pub(crate) fn new(edges: &'a [(CfgEdge, ArmProvenance)]) -> Self {
        #[cfg(test)]
        super::work_tests::index_built();
        let mut by_source: BTreeMap<Line, Vec<usize>> = BTreeMap::new();
        for (ordinal, (edge, _)) in edges.iter().enumerate() {
            by_source.entry(edge.from_line).or_default().push(ordinal);
        }
        Self { edges, by_source }
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.edges.len()
    }

    pub(super) fn for_owner(&self, line_index: &BTreeMap<Line, usize>) -> Vec<SelectedEdge> {
        #[cfg(test)]
        if tests::legacy_enabled() {
            let selected = self.legacy_for_owner(line_index);
            tests::record_edges(&selected);
            return selected;
        }
        let mut ordinals = Vec::new();
        // Each unique source line visits its bucket once. Do not partition by
        // function identity: a nested owner's lines can admit another's edges.
        for (line, &from) in line_index {
            if let Some(bucket) = self.by_source.get(line) {
                for &ordinal in bucket {
                    #[cfg(test)]
                    super::work_tests::bump(|counts| counts.cfg_candidates += 1);
                    let edge = &self.edges[ordinal].0;
                    if let Some(&to) = line_index.get(&edge.to_line) {
                        ordinals.push((ordinal, from, to));
                    }
                }
            }
        }
        // Ordinals are unique even when endpoint pairs/provenance are equal.
        // Sorting restores file order; nothing is deduplicated.
        ordinals.sort_unstable_by_key(|entry| entry.0);
        let selected = ordinals
            .into_iter()
            .map(|(ordinal, from, to)| {
                let (edge, provenance) = self.edges[ordinal].clone();
                #[cfg(test)]
                {
                    super::work_tests::bump(|counts| counts.cfg_cloned += 1);
                    record_admitted(provenance);
                }
                (from, to, edge, provenance)
            })
            .collect::<Vec<_>>();
        #[cfg(test)]
        tests::record_edges(&selected);
        selected
    }

    /// Frozen main cloning/admission loop, used only as a differential oracle.
    #[cfg(test)]
    fn legacy_for_owner(&self, line_index: &BTreeMap<Line, usize>) -> Vec<SelectedEdge> {
        let mut selected = Vec::new();
        for (edge, provenance) in self.edges.iter().cloned() {
            super::work_tests::bump(|counts| {
                counts.cfg_candidates += 1;
                counts.cfg_cloned += 1;
            });
            let (Some(&from), Some(&to)) = (
                line_index.get(&edge.from_line),
                line_index.get(&edge.to_line),
            ) else {
                continue;
            };
            record_admitted(provenance);
            selected.push((from, to, edge, provenance));
        }
        selected
    }
}

#[cfg(test)]
fn record_admitted(provenance: ArmProvenance) {
    use crate::cfg::EdgeOrigin;
    super::work_tests::bump(|counts| {
        counts.cfg_admitted += 1;
        match provenance.origin {
            EdgeOrigin::Structured => counts.cfg_structured += 1,
            EdgeOrigin::Sequential if provenance.crosses_lexical_arm() => counts.cfg_cross_arm += 1,
            EdgeOrigin::Sequential => counts.cfg_sequential += 1,
        }
    });
}

#[cfg(test)]
pub(super) mod tests;
