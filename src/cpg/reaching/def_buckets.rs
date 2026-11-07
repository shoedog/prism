//! Ordered definition indexes, scoped to one admitted RD invocation.
use super::{AccessPath, BitSet, DefSite, FlowEdge, Line};
use crate::access_path::ARRAY_FIELD;
use std::collections::BTreeMap;

pub(super) struct DefinitionBuckets<'a> {
    #[cfg(test)]
    defs: &'a [DefSite],
    /// Defs `..n_recorded` are the producer's; the rest are census kill-only defs (#660 S1b-1): they
    /// gen and kill but are never candidates for an edge and are never killed.
    n_recorded: usize,
    /// Recorded, non-alias defs by base name (the census kills by `affects`, not by path equality).
    by_base: BTreeMap<&'a str, Vec<usize>>,
    // AccessPath derives Eq and Ord from the same complete base/fields value.
    kill_paths: BTreeMap<&'a AccessPath, Vec<usize>>,
    /// #674 (JS/TS/TSX owners only): recorded, non-alias defs by base, as a trie over their fields.
    tries: BTreeMap<&'a str, Trie<'a>>,
    /// JS/TS/TSX owners: a recorded write kills by `affects` (#674). Otherwise main's path equality.
    recorded_affects: bool,
    exact: BTreeMap<(&'a AccessPath, Line, usize), Vec<usize>>,
    fallback: BTreeMap<(&'a AccessPath, Line), Vec<usize>>,
}

impl<'a> DefinitionBuckets<'a> {
    #[cfg(test)]
    pub(super) fn new(defs: &'a [DefSite]) -> Self {
        Self::with_kill_only(defs, defs.len(), false)
    }

    pub(super) fn with_kill_only(
        defs: &'a [DefSite],
        n_recorded: usize,
        recorded_affects: bool,
    ) -> Self {
        let mut buckets = Self {
            #[cfg(test)]
            defs,
            n_recorded,
            by_base: BTreeMap::new(),
            kill_paths: BTreeMap::new(),
            tries: BTreeMap::new(),
            recorded_affects,
            exact: BTreeMap::new(),
            fallback: BTreeMap::new(),
        };
        // Appending in input order preserves the scanner's ascending indices.
        for (index, def) in defs.iter().enumerate().take(n_recorded) {
            if !def.alias_derived && n_recorded < defs.len() {
                buckets
                    .by_base
                    .entry(def.path.base.as_str())
                    .or_default()
                    .push(index);
            }
            if !def.alias_derived {
                buckets.kill_paths.entry(&def.path).or_default().push(index);
                if recorded_affects {
                    buckets
                        .tries
                        .entry(def.path.base.as_str())
                        .or_default()
                        .insert(&def.path.fields, index);
                }
            }
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
        buckets
    }

    pub(super) fn matching_defs(&self, edge: &FlowEdge) -> Vec<usize> {
        #[cfg(test)]
        if super::def_bucket_tests::legacy_enabled() {
            return super::matching_defs(edge, &self.defs[..self.n_recorded]);
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
        binding_facts: &crate::cpg::binding_facts::BindingFacts,
    ) -> (Vec<BitSet>, Vec<BitSet>) {
        #[cfg(test)]
        if super::def_bucket_tests::legacy_enabled() {
            return legacy_kill_sets_with(
                defs,
                gen,
                binding_facts,
                self.n_recorded,
                self.recorded_affects,
            );
        }
        let mut kill = vec![BitSet::new(defs.len()); gen.len()];
        let mut flat_kill = vec![BitSet::new(defs.len()); gen.len()];
        for node in 0..gen.len() {
            for new_def in gen[node].members() {
                if defs[new_def].alias_derived {
                    continue;
                }
                if new_def >= self.n_recorded {
                    self.kill_only_kills(
                        defs,
                        gen,
                        binding_facts,
                        node,
                        new_def,
                        &mut kill,
                        &mut flat_kill,
                    );
                    continue;
                }
                if self.recorded_affects {
                    self.recorded_affects_kills(
                        defs,
                        gen,
                        binding_facts,
                        node,
                        new_def,
                        &mut kill,
                        &mut flat_kill,
                    );
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

    /// #674: a recorded write `w` kills every recorded def of its base that it `affects` (DESIGN 4.4),
    /// in a JS/TS/TSX owner. Path equality is the special case, so main's kills are a subset.
    ///
    /// Cost: the base's defs sit in a trie over their fields, and `w` walks only the branches it can
    /// match (its own component and `[]`, or every child under a `[]` of its own), then takes everything
    /// below. Work is the number of affected defs plus the trie nodes on the way, never a scan of the
    /// base's other defs (a `this` base holds hundreds).
    #[allow(clippy::too_many_arguments)]
    fn recorded_affects_kills(
        &self,
        defs: &[DefSite],
        gen: &[BitSet],
        binding_facts: &crate::cpg::binding_facts::BindingFacts,
        node: usize,
        new_def: usize,
        kill: &mut [BitSet],
        flat_kill: &mut [BitSet],
    ) {
        let w = &defs[new_def].path;
        let Some(trie) = self.tries.get(w.base.as_str()) else {
            return;
        };
        trie.affected(&w.fields, &mut |old_def| {
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
        });
    }

    /// A census write kills every recorded def of its base that it `affects` (A2): the recorded paths
    /// at or below its own, a `[]` component matching any component. Binding as for recorded defs.
    #[allow(clippy::too_many_arguments)]
    fn kill_only_kills(
        &self,
        defs: &[DefSite],
        gen: &[BitSet],
        binding_facts: &crate::cpg::binding_facts::BindingFacts,
        node: usize,
        new_def: usize,
        kill: &mut [BitSet],
        flat_kill: &mut [BitSet],
    ) {
        let Some(olds) = self.by_base.get(defs[new_def].path.base.as_str()) else {
            return;
        };
        for &old_def in olds {
            if !gen[node].contains(old_def) && affects(&defs[new_def].path, &defs[old_def].path) {
                flat_kill[node].insert(old_def);
                if binding_facts.same_def_binding(new_def, old_def) {
                    kill[node].insert(old_def);
                }
            }
        }
    }
}

/// Defs of one base by field path (#674). `affected(w)` yields exactly the defs `q` with `affects(w, q)`.
#[derive(Default)]
pub(super) struct Trie<'a> {
    defs: Vec<usize>,
    children: BTreeMap<&'a str, Trie<'a>>,
}

impl<'a> Trie<'a> {
    pub(super) fn insert(&mut self, fields: &'a [String], def: usize) {
        match fields.split_first() {
            None => self.defs.push(def),
            Some((field, rest)) => self
                .children
                .entry(field.as_str())
                .or_default()
                .insert(rest, def),
        }
    }

    /// The defs at or below `fields`, a `[]` component on either side matching any component.
    pub(super) fn affected(&self, fields: &[String], visit: &mut dyn FnMut(usize)) {
        match fields.split_first() {
            None => self.all(visit),
            Some((field, rest)) if field == ARRAY_FIELD => {
                for child in self.children.values() {
                    child.affected(rest, visit);
                }
            }
            Some((field, rest)) => {
                for key in [field.as_str(), ARRAY_FIELD] {
                    if let Some(child) = self.children.get(key) {
                        child.affected(rest, visit);
                    }
                }
            }
        }
    }

    fn all(&self, visit: &mut dyn FnMut(usize)) {
        self.defs.iter().for_each(|&def| visit(def));
        for child in self.children.values() {
            child.all(visit);
        }
    }
}

/// `w` affects `q`: same base and `w`'s fields are a prefix of `q`'s, `[]` matching any field on
/// either side (DESIGN.md 4.4).
pub(super) fn affects(w: &AccessPath, q: &AccessPath) -> bool {
    w.base == q.base
        && w.fields.len() <= q.fields.len()
        && w.fields.iter().zip(&q.fields).all(|(a, b)| {
            a == b || a == crate::access_path::ARRAY_FIELD || b == crate::access_path::ARRAY_FIELD
        })
}

// Frozen main scanner: independent of all bucket keys and construction.
#[cfg(test)]
pub(super) fn legacy_kill_sets(
    defs: &[DefSite],
    gen: &[BitSet],
    binding_facts: &crate::cpg::binding_facts::BindingFacts,
) -> (Vec<BitSet>, Vec<BitSet>) {
    legacy_kill_sets_with(defs, gen, binding_facts, defs.len(), false)
}

/// The all-pairs scan with census kill-only defs (`n_recorded..`): equal-path kills among recorded
/// defs as on main (`affects` kills when `recorded_affects`, #674), `affects` kills from kill-only
/// defs, kill-only defs never killed.
#[cfg(test)]
pub(super) fn legacy_kill_sets_with(
    defs: &[DefSite],
    gen: &[BitSet],
    binding_facts: &crate::cpg::binding_facts::BindingFacts,
    n_recorded: usize,
    recorded_affects: bool,
) -> (Vec<BitSet>, Vec<BitSet>) {
    let mut kill = vec![BitSet::new(defs.len()); gen.len()];
    let mut flat_kill = vec![BitSet::new(defs.len()); gen.len()];
    for node in 0..gen.len() {
        let generated: Vec<usize> = gen[node].members().collect();
        for &new_def in &generated {
            if defs[new_def].alias_derived {
                continue;
            }
            for old_def in 0..n_recorded {
                #[cfg(test)]
                super::work_tests::bump(|counts| counts.definition_pair_comparisons += 1);
                let related = if new_def >= n_recorded || recorded_affects {
                    affects(&defs[new_def].path, &defs[old_def].path)
                } else {
                    defs[old_def].path == defs[new_def].path
                };
                #[cfg(test)]
                if !defs[old_def].alias_derived && related {
                    super::work_tests::bump(|counts| {
                        counts.projected_definition_pair_comparisons += 1
                    });
                }
                if !gen[node].contains(old_def) && !defs[old_def].alias_derived && related {
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
