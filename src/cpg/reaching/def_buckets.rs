//! Ordered definition indexes, scoped to one admitted RD invocation.
use super::{scope, AccessPath, BitSet, DefSite, FlowEdge, Line};
use std::collections::BTreeMap;

pub(super) struct DefinitionBuckets<'a> {
    #[cfg(test)]
    defs: &'a [DefSite],
    // AccessPath derives Eq and Ord from the same complete base/fields value.
    kill_paths: BTreeMap<&'a AccessPath, Vec<usize>>,
    exact: BTreeMap<(&'a AccessPath, Line, usize), Vec<usize>>,
    fallback: BTreeMap<(&'a AccessPath, Line), Vec<usize>>,
}

impl<'a> DefinitionBuckets<'a> {
    pub(super) fn new(defs: &'a [DefSite]) -> Self {
        let mut buckets = Self {
            #[cfg(test)]
            defs,
            kill_paths: BTreeMap::new(),
            exact: BTreeMap::new(),
            fallback: BTreeMap::new(),
        };
        // Appending in input order preserves the scanner's ascending indices.
        for (index, def) in defs.iter().enumerate() {
            if !def.alias_derived {
                buckets.kill_paths.entry(&def.path).or_default().push(index);
            }
            if !def.synthetic {
                buckets
                    .exact
                    .entry((&def.path, def.line, def.start_byte))
                    .or_default()
                    .push(index);
                buckets
                    .fallback
                    .entry((&def.path, def.line))
                    .or_default()
                    .push(index);
            }
        }
        buckets
    }

    pub(super) fn matching_defs(&self, edge: &FlowEdge) -> Vec<usize> {
        #[cfg(test)]
        if super::def_bucket_tests::legacy_enabled() {
            return super::matching_defs(edge, self.defs);
        }
        let exact = self
            .exact
            .get(&(&edge.from.path, edge.from.line, edge.from.start_byte));
        let matches = if let Some(exact) = exact.filter(|indices| !indices.is_empty()) {
            exact.as_slice()
        } else {
            #[cfg(test)]
            super::work_tests::bump(|counts| counts.matching_defs_fallbacks += 1);
            self.fallback
                .get(&(&edge.from.path, edge.from.line))
                .map(Vec::as_slice)
                .unwrap_or(&[])
        };
        #[cfg(test)]
        super::work_tests::bump(|counts| {
            counts.matching_defs_calls += 1;
            counts.matching_defs_items += matches.len();
            counts.projected_matching_defs_items += matches.len();
        });
        matches.to_vec()
    }

    pub(super) fn kill_sets(
        &self,
        defs: &[DefSite],
        gen: &[BitSet],
        binding_facts: &scope::BindingFacts,
    ) -> (Vec<BitSet>, Vec<BitSet>) {
        #[cfg(test)]
        if super::def_bucket_tests::legacy_enabled() {
            return legacy_kill_sets(defs, gen, binding_facts);
        }
        let mut kill = vec![BitSet::new(defs.len()); gen.len()];
        let mut flat_kill = vec![BitSet::new(defs.len()); gen.len()];
        for node in 0..gen.len() {
            for new_def in gen[node].members() {
                if defs[new_def].alias_derived {
                    continue;
                }
                for &old_def in &self.kill_paths[&defs[new_def].path] {
                    #[cfg(test)]
                    super::work_tests::bump(|counts| {
                        counts.definition_pair_comparisons += 1;
                        counts.projected_definition_pair_comparisons += 1;
                    });
                    if !gen[node].contains(old_def) {
                        flat_kill[node].insert(old_def);
                        if binding_facts.same_def_binding(new_def, old_def) {
                            kill[node].insert(old_def);
                        }
                    }
                }
            }
        }
        (kill, flat_kill)
    }
}

// Frozen main scanner: independent of all bucket keys and construction.
#[cfg(test)]
pub(super) fn legacy_kill_sets(
    defs: &[DefSite],
    gen: &[BitSet],
    binding_facts: &scope::BindingFacts,
) -> (Vec<BitSet>, Vec<BitSet>) {
    let mut kill = vec![BitSet::new(defs.len()); gen.len()];
    let mut flat_kill = vec![BitSet::new(defs.len()); gen.len()];
    for node in 0..gen.len() {
        let generated: Vec<usize> = gen[node].members().collect();
        for &new_def in &generated {
            if defs[new_def].alias_derived {
                continue;
            }
            for old_def in 0..defs.len() {
                #[cfg(test)]
                super::work_tests::bump(|counts| counts.definition_pair_comparisons += 1);
                #[cfg(test)]
                if !defs[old_def].alias_derived && defs[old_def].path == defs[new_def].path {
                    super::work_tests::bump(|counts| {
                        counts.projected_definition_pair_comparisons += 1
                    });
                }
                if !gen[node].contains(old_def)
                    && !defs[old_def].alias_derived
                    && defs[old_def].path == defs[new_def].path
                {
                    flat_kill[node].insert(old_def);
                    if binding_facts.same_def_binding(new_def, old_def) {
                        kill[node].insert(old_def);
                    }
                }
            }
        }
    }

    (kill, flat_kill)
}
