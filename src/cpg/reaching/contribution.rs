//! Independent field-contribution RD. Ordinary gen/kill and labels are immutable.
use super::{
    go_aggregate::{Facts, Occurrence},
    graph::BitSet,
    recall_stats::Status,
    scope::{BindingFacts, BindingRelation},
    solve_reaching_sets,
    statement_lookup::StatementLookup,
    DefSite, Line, RdFileStats, RdUnavailable,
};
use crate::{
    access_path::AccessPath,
    ast::ParsedFile,
    cpg::{FlowConfidence, FlowDoubt},
    data_flow::{FlowEdge, VarAccessKind, VarLocation},
    languages::Language,
};
use std::collections::BTreeMap;
use tree_sitter::Node;

const ADDITION_CAP: usize = 1_000_000;
#[cfg(test)]
thread_local! { static TEST_CAP: std::cell::Cell<usize> = const { std::cell::Cell::new(ADDITION_CAP) }; }
#[cfg(test)]
pub(crate) fn with_cap<T>(cap: usize, f: impl FnOnce() -> T) -> T {
    struct Reset(usize);
    impl Drop for Reset {
        fn drop(&mut self) {
            TEST_CAP.with(|c| c.set(self.0));
        }
    }
    let _reset = Reset(TEST_CAP.with(|c| c.replace(cap)));
    f()
}
fn cap() -> usize {
    #[cfg(test)]
    {
        TEST_CAP.with(|c| c.get())
    }
    #[cfg(not(test))]
    {
        ADDITION_CAP
    }
}

type Locations = BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>>;
type Labels = BTreeMap<(VarLocation, VarLocation), FlowConfidence>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RecallResult {
    status: Status,
    edges: Vec<(FlowEdge, FlowConfidence)>,
}
impl RecallResult {
    /// Admission already succeeded before either edges or supplemental Uses
    /// were allocated. Reuse existing occurrences whenever the consumer has one.
    pub(crate) fn emit(
        self,
        uses: &mut Locations,
        edges: &mut Vec<FlowEdge>,
        labels: &mut Labels,
        stats: &mut RdFileStats,
        owner: Node<'_>,
        name: &str,
    ) {
        stats.recall.record(
            (
                name.to_owned(),
                owner.start_position().row + 1,
                owner.start_byte(),
            ),
            self.status,
        );
        for (mut edge, label) in self.edges {
            let to = &edge.to;
            let locations = uses
                .entry((
                    to.file.clone(),
                    to.function.clone(),
                    to.function_start_line,
                    to.path.clone(),
                ))
                .or_default();
            if let Some(existing) = locations.iter().find(|u| {
                u.line == to.line && u.start_byte == to.start_byte && u.end_byte == to.end_byte
            }) {
                edge.to = existing.clone();
            } else {
                locations.push(to.clone());
            }
            labels
                .entry((edge.from.clone(), edge.to.clone()))
                .and_modify(|old| *old = old.worst(label))
                .or_insert(label);
            edges.push(edge);
        }
    }
}

pub(crate) fn record_skip(
    parsed: &ParsedFile,
    owner: Node<'_>,
    defs: &[DefSite],
    reason: RdUnavailable,
    stats: &mut RdFileStats,
) {
    if parsed.language != Language::Go
        || !defs.iter().any(|d| !d.alias_derived && d.path.has_fields())
    {
        return;
    }
    let facts = Facts::new(parsed, owner);
    if !defs.iter().any(|d| {
        !d.alias_derived
            && d.path.has_fields()
            && facts.writes.contains_key(&(d.path.clone(), d.start_byte))
            && facts.reads.iter().any(|r| {
                r.line != d.line
                    && r.path.base == d.path.base
                    && r.path.fields.len() < d.path.fields.len()
                    && d.path.fields.starts_with(&r.path.fields)
            })
    }) {
        return;
    }
    let status = match reason {
        RdUnavailable::NoCfgEdges => Status::NoCfg,
        _ => Status::DefinitionsOrStatementsCap,
    };
    stats.recall.record(
        (
            parsed.dfg_owner_name(&owner).unwrap(),
            owner.start_position().row + 1,
            owner.start_byte(),
        ),
        status,
    );
}

struct Lane<'a> {
    defs: &'a [DefSite],
    reads: &'a [Occurrence],
    mapped_defs: &'a [Option<usize>],
    mapped_reads: Vec<Option<usize>>,
    bindings: &'a BindingFacts,
    incoming: Vec<BitSet>,
    reachable: &'a BitSet,
    exact: super::DefinitionReachability<'a>,
}
impl Lane<'_> {
    fn classify(&mut self, d: usize, u: usize) -> Option<FlowConfidence> {
        let def = &self.defs[d];
        let read = &self.reads[u];
        // S2 owns same-line ordering. No capture facts enter this lane.
        if def.line == read.line {
            return None;
        }
        let binding_uncertain = match self.bindings.relation_at(&read.path.base, read.start, d) {
            BindingRelation::KilledAt(_) => return None,
            BindingRelation::Unresolved => true,
            BindingRelation::Same => false,
        };
        let (Some(from), Some(to)) = (self.mapped_defs[d], self.mapped_reads[u]) else {
            return (def.line < read.line)
                .then_some(FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete));
        };
        if !self.incoming[to].contains(d) {
            return None;
        }
        let exact = !binding_uncertain
            && !self
                .bindings
                .lookup_at_requires_flat_fallback(&read.path.base, read.start, d)
            && self.reachable.contains(from)
            && self.exact.reaches(d, to)
            && from != to
            && self
                .bindings
                .unclassified_binding_lines(&read.path.base)
                .is_empty();
        Some(if exact {
            FlowConfidence::Exact
        } else {
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)
        })
    }

    /// The only survivor iterator, used unchanged for counting and replay.
    /// Jobs index path-prefix buckets; never allocate a prospective pair list.
    fn survivors<'a>(
        &'a mut self,
        jobs: &'a [(usize, usize, &[usize])],
    ) -> impl Iterator<Item = (usize, usize, FlowConfidence)> + 'a {
        let (mut job, mut occurrence, mut duplicate) = (0, 0, 0);
        std::iter::from_fn(move || loop {
            let &(d, multiplicity, targets) = jobs.get(job)?;
            if occurrence == targets.len() {
                occurrence = 0;
                duplicate = 0;
                job += 1;
                continue;
            }
            let u = targets[occurrence];
            duplicate += 1;
            if duplicate == multiplicity {
                duplicate = 0;
                occurrence += 1;
            }
            if let Some(label) = self.classify(d, u) {
                return Some((d, u, label));
            }
        })
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn analyze(
    parsed: &ParsedFile,
    owner: Node<'_>,
    raw_defs: &[DefSite],
    defs: &[DefSite],
    mapped_defs: &[Option<usize>],
    line_index: &BTreeMap<Line, usize>,
    statements: &mut StatementLookup<'_>,
    successors: &[Vec<(usize, bool)>],
    predecessors: &[Vec<usize>],
    reachable: &BitSet,
    bindings: &BindingFacts,
) -> Option<RecallResult> {
    if parsed.language != Language::Go
        || !defs.iter().any(|d| d.path.has_fields() && !d.alias_derived)
    {
        return None;
    }
    let facts = Facts::new(parsed, owner);
    let mut sources = BTreeMap::<AccessPath, Vec<usize>>::new();
    let mut multiplicity = BTreeMap::new();
    for raw in raw_defs.iter().filter(|d| !d.alias_derived) {
        *multiplicity
            .entry((&raw.path, raw.start_byte))
            .or_insert(0usize) += 1;
    }
    for (i, d) in defs.iter().enumerate() {
        if !d.alias_derived
            && d.path.has_fields()
            && facts.writes.contains_key(&(d.path.clone(), d.start_byte))
            // S2 owns ordering within collapsed CFG nodes. In particular,
            // never regenerate a field killed later on its own source line.
            && !defs.iter().enumerate().any(|(k, w)| k != i && !w.alias_derived
                && mapped_defs[i].is_some() && mapped_defs[k] == mapped_defs[i] && w.path.base == d.path.base
                && d.path.fields.starts_with(&w.path.fields)
                && facts.writes.contains_key(&(w.path.clone(), w.start_byte))
                && bindings.same_def_binding(i, k))
        {
            sources.entry(d.path.clone()).or_default().push(i);
        }
    }
    if sources.is_empty() {
        return None;
    }
    let mut read_buckets = BTreeMap::<AccessPath, Vec<usize>>::new();
    for (i, read) in facts.reads.iter().enumerate() {
        read_buckets.entry(read.path.clone()).or_default().push(i);
    }
    let mut jobs = Vec::new();
    for (path, indices) in &sources {
        for depth in 0..path.fields.len() {
            let prefix = AccessPath::with_fields(path.base.clone(), path.fields[..depth].to_vec());
            if let Some(targets) = read_buckets.get(&prefix) {
                for &d in indices {
                    jobs.push((
                        d,
                        multiplicity[&(&defs[d].path, defs[d].start_byte)],
                        targets.as_slice(),
                    ));
                }
            }
        }
    }
    if jobs.is_empty() {
        return None;
    }
    let nodes = successors.len();
    let mut gen = vec![BitSet::new(defs.len()); nodes];
    let mut kill = vec![BitSet::new(defs.len()); nodes];
    // Index each source by its ancestors. Genuine writes kill only comparable
    // contributions of the same lexical root binding, in this lane alone.
    let mut descendants = BTreeMap::<AccessPath, Vec<usize>>::new();
    for (path, indices) in &sources {
        for &d in indices {
            if let Some(node) = mapped_defs[d] {
                gen[node].insert(d);
            }
        }
        for depth in 0..=path.fields.len() {
            descendants
                .entry(AccessPath::with_fields(
                    path.base.clone(),
                    path.fields[..depth].to_vec(),
                ))
                .or_default()
                .extend(indices);
        }
    }
    for (k, write) in defs.iter().enumerate() {
        if write.alias_derived
            || !facts
                .writes
                .contains_key(&(write.path.clone(), write.start_byte))
        {
            continue;
        }
        if let (Some(node), Some(killed)) = (mapped_defs[k], descendants.get(&write.path)) {
            for &d in killed {
                if bindings.proven_same_def_binding(d, k) {
                    kill[node].insert(d);
                }
            }
        }
    }
    let (incoming, _) = solve_reaching_sets(&gen, &kill, predecessors, successors, 0, defs.len());
    drop(gen);
    let mapped_reads = facts
        .reads
        .iter()
        .map(|r| {
            statements
                .get(r.line)
                .and_then(|line| line_index.get(&line).copied())
        })
        .collect();
    let mut lane = Lane {
        defs,
        reads: &facts.reads,
        mapped_defs,
        mapped_reads,
        bindings,
        incoming,
        reachable,
        exact: super::DefinitionReachability::new(successors, &kill, mapped_defs),
    };
    let count = lane
        .survivors(&jobs)
        .try_fold(0usize, |n, _| n.checked_add(1))
        .expect("DFG recall candidate count overflow");
    if count == 0 {
        return None;
    }
    if count > cap() {
        return Some(RecallResult {
            status: Status::AdditionCap(count),
            edges: vec![],
        });
    }
    let name = parsed.dfg_owner_name(&owner).unwrap();
    let make_loc = |path: &AccessPath, line, start_byte, end_byte, kind| VarLocation {
        file: parsed.path.clone(),
        function: name.clone(),
        function_start_line: owner.start_position().row + 1,
        path: path.clone(),
        line,
        start_byte,
        end_byte,
        kind,
    };
    let edges = lane
        .survivors(&jobs)
        .map(|(d, u, label)| {
            let def = &defs[d];
            let read = &facts.reads[u];
            (
                FlowEdge {
                    from: make_loc(
                        &def.path,
                        def.line,
                        def.start_byte,
                        facts.writes[&(def.path.clone(), def.start_byte)],
                        VarAccessKind::Def,
                    ),
                    to: make_loc(
                        &read.path,
                        read.line,
                        read.start,
                        read.end,
                        VarAccessKind::Use,
                    ),
                },
                label,
            )
        })
        .collect();
    Some(RecallResult {
        status: Status::Admitted(count),
        edges,
    })
}

#[cfg(test)]
#[path = "contribution_tests.rs"]
mod tests;
