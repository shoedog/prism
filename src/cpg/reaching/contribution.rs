//! Independent field-contribution RD. Ordinary gen/kill and labels are immutable.
use super::{
    go_aggregate::{Facts, Occurrence},
    graph::BitSet,
    recall_stats::{ReceiverEdge, ReceiverKey, Status},
    route::{order, Order, Pos},
    solve_reaching_sets,
    statement_lookup::StatementLookup,
    DefSite, Line, RdFileStats, RdUnavailable,
};
use crate::{
    access_path::AccessPath,
    ast::ParsedFile,
    cpg::{
        binding_facts::{BindingFacts, BindingRelation},
        FlowConfidence, FlowDoubt,
    },
    data_flow::{FlowEdge, VarAccessKind, VarLocation},
    languages::Language,
};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

#[path = "contribution_capture.rs"]
mod capture;

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
type LaneGraph = (Vec<Vec<(usize, bool)>>, Vec<Vec<usize>>);
type Labels = BTreeMap<(VarLocation, VarLocation), FlowConfidence>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RecallResult {
    status: Status,
    edges: Vec<(FlowEdge, FlowConfidence, Option<String>)>,
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
        let mut created = BTreeSet::new();
        for (mut edge, label, selector) in self.edges {
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
                created.insert((to.start_byte, to.end_byte));
            }
            if let Some(selector) = selector {
                stats.recall.record_receiver(
                    ReceiverKey {
                        from_start: edge.from.start_byte,
                        from_end: edge.from.end_byte,
                        from_path: edge.from.path.clone(),
                        to_start: edge.to.start_byte,
                        to_end: edge.to.end_byte,
                    },
                    ReceiverEdge {
                        selector,
                        label,
                        new_use: created.contains(&(edge.to.start_byte, edge.to.end_byte)),
                        refused: None,
                    },
                );
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
    facts: &'a Facts,
    reads: &'a [Occurrence],
    mapped_defs: &'a [Option<usize>],
    mapped_reads: Vec<Option<usize>>,
    bindings: &'a BindingFacts,
    incoming: Vec<BitSet>,
    reachable: &'a BitSet,
    exact: super::DefinitionReachability<'a>,
    gen: &'a [BitSet],
    kill: &'a [BitSet],
    successors: &'a [Vec<(usize, bool)>],
    /// Sources a later write of the same node kills: never regenerated at
    /// their node, so only a read of the same node can still see them.
    later_killed: BTreeSet<usize>,
    /// Genuine write definitions per line, for same-line kills.
    line_writes: BTreeMap<usize, Vec<usize>>,
    /// Evaluation position of each genuine write.
    def_pos: Vec<Option<&'a Pos>>,
    /// Genuine writes and field-write sources by CFG node.
    node_writes: BTreeMap<usize, Vec<usize>>,
    node_sources: BTreeMap<usize, Vec<usize>>,
    /// Source line of each CFG node.
    node_line: Vec<usize>,
    /// CFG node where each outermost function literal is created, and where
    /// each call of a locally bound literal happens.
    closure_nodes: Vec<Option<usize>>,
    call_nodes: Vec<Vec<Option<usize>>>,
    capture_live: BTreeMap<usize, BitSet>,
    deferred_live: BTreeMap<usize, BitSet>,
}
impl<'l> Lane<'l> {
    /// Whether the read's root is the binding the source wrote (`None`: a
    /// different binding or a kill).
    fn binding_uncertain(&self, d: usize, u: usize) -> Option<bool> {
        let read = &self.reads[u];
        if read.receiver.is_some()
            && self
                .bindings
                .lookup_at_requires_flat_fallback(&read.path.base, read.start, d)
        {
            return None;
        }
        match self.bindings.relation_at(&read.path.base, read.start, d) {
            BindingRelation::KilledAt(_) => None,
            // A receiver must be a proven variable binding.
            BindingRelation::Unresolved if read.receiver.is_some() => None,
            BindingRelation::Unresolved => Some(true),
            BindingRelation::Same => Some(false),
        }
    }

    fn classify(&mut self, d: usize, u: usize) -> Option<FlowConfidence> {
        if let Some(closure) = self.reads[u].closure {
            return self.classify_capture(d, u, closure);
        }
        if self.reads[u].deferred {
            return self.classify_deferred(d, u);
        }
        let def = &self.defs[d];
        let read = &self.reads[u];
        if def.line == read.line {
            return self.classify_same_line(d, u);
        }
        if self.later_killed.contains(&d) {
            return None;
        }
        let mapped = (self.mapped_defs[d], self.mapped_reads[u]);
        // Cheapest test first: no contribution reaches the use node at all.
        if let (Some(_), Some(to)) = mapped {
            if !self.incoming[to].contains(d) {
                return None;
            }
        }
        let binding_uncertain = self.binding_uncertain(d, u)?;
        let (Some(from), Some(to)) = mapped else {
            return (def.line < read.line)
                .then_some(FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete));
        };
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
        if exact {
            Some(FlowConfidence::Exact)
        } else if read.line < def.line {
            // A use that precedes its def needs a proven cyclic route (a loop
            // back edge); an uncertain route proves nothing.
            None
        } else {
            Some(FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete))
        }
    }

    /// The only survivor iterator, used unchanged for counting and replay.
    /// Jobs index path-prefix buckets; never allocate a prospective pair list.
    fn survivors<'a>(
        &'a mut self,
        jobs: &'a [(usize, usize, &[usize])],
    ) -> impl Iterator<Item = (usize, usize, FlowConfidence)> + use<'a, 'l>
    where
        'l: 'a,
    {
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
    let mut later_killed = BTreeSet::new();
    let pos_of = |path: &AccessPath, start: usize| &facts.write_meta[&(path.clone(), start)].pos;
    for (i, d) in defs.iter().enumerate() {
        if !d.alias_derived
            && d.path.has_fields()
            && facts.writes.contains_key(&(d.path.clone(), d.start_byte))
        {
            // A write of the same node that runs after (or in the statement
            // of) this one kills this field before the node ends (never
            // regenerate it past its own line); an earlier one
            // (`p = q; p.f = x`) or one in another arm leaves it live.
            if defs.iter().enumerate().any(|(k, w)| {
                k != i
                    && !w.alias_derived
                    && mapped_defs[i].is_some()
                    && mapped_defs[k] == mapped_defs[i]
                    && w.path.base == d.path.base
                    && d.path.fields.starts_with(&w.path.fields)
                    && facts.writes.contains_key(&(w.path.clone(), w.start_byte))
                    && matches!(
                        order(pos_of(&d.path, d.start_byte), pos_of(&w.path, w.start_byte)),
                        Order::Before | Order::Same
                    )
                    && bindings.same_def_binding(i, k)
            }) {
                later_killed.insert(i);
            }
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
            if let (Some(node), false) = (mapped_defs[d], later_killed.contains(&d)) {
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
                if bindings.proven_same_def_binding(d, k) || bindings.same_fallback_binding(d, k) {
                    kill[node].insert(d);
                }
            }
        }
    }
    // Range, receive and type-switch rebindings carry no DefSite. Each kills
    // every contribution of the binding it (re)writes, at its own node; a
    // range assignment applies on iteration entry only, so it gets a virtual
    // node on the header's body-entry edges and the zero-iteration exit keeps
    // the old value.
    let mut ext: Option<LaneGraph> = None;
    if !owner.has_error() {
        if let Some(body) = owner.child_by_field_name("body") {
            let headers = super::go_header_writes::collect(parsed, body);
            let mut node_line = vec![0usize; nodes];
            for (&line, &n) in line_index {
                if n < nodes {
                    node_line[n] = line;
                }
            }
            // One virtual node per header node, carrying the union of every
            // written name's kills: parallel nodes would let each name's
            // contributions bypass the other's kill.
            let mut groups = BTreeMap::<(usize, Option<usize>), (usize, Vec<usize>)>::new();
            for h in headers {
                let Some(node) = statements
                    .get(h.line)
                    .and_then(|line| line_index.get(&line).copied())
                else {
                    continue;
                };
                // Same kill as an ordinary assignment to this path: the same
                // path, a prefix of the contribution, the root, `*root`.
                let killed = sources.values().flatten().copied().filter(|&d| {
                    defs[d].path.base == h.path.base
                        && defs[d].path.fields.starts_with(&h.path.fields)
                        && bindings.relation_at(&h.path.base, h.probe, d) == BindingRelation::Same
                });
                let last_line = match h.entry {
                    super::go_header_writes::Entry::Range { last_line } => Some(last_line),
                    super::go_header_writes::Entry::Anchor => None,
                };
                let slot = groups.entry((node, last_line)).or_insert((h.line, vec![]));
                slot.1.extend(killed);
            }
            for ((node, last_line), (line, mut killed)) in groups {
                killed.sort_unstable();
                killed.dedup();
                if killed.is_empty() {
                    continue;
                }
                let mut at = node;
                if let Some(last_line) = last_line {
                    let body_edges: Vec<(usize, bool)> = successors[node]
                        .iter()
                        .copied()
                        .filter(|&(t, _)| node_line[t] > line && node_line[t] <= last_line)
                        .collect();
                    if !body_edges.is_empty() {
                        let (es, ep) =
                            ext.get_or_insert_with(|| (successors.to_vec(), predecessors.to_vec()));
                        let v = es.len();
                        es.push(Vec::new());
                        ep.push(vec![node]);
                        gen.push(BitSet::new(defs.len()));
                        kill.push(BitSet::new(defs.len()));
                        for (t, flag) in body_edges {
                            es[node].retain(|e| e.0 != t);
                            ep[t].retain(|&q| q != node);
                            es[v].push((t, false));
                            ep[t].push(v);
                            es[node].push((v, flag));
                        }
                        at = v;
                    }
                }
                for d in killed {
                    kill[at].insert(d);
                }
            }
        }
    }
    let (successors, predecessors) = match &ext {
        Some((s, p)) => (s.as_slice(), p.as_slice()),
        None => (successors, predecessors),
    };
    let (incoming, _) = solve_reaching_sets(&gen, &kill, predecessors, successors, 0, defs.len());
    let mapped_reads = facts
        .reads
        .iter()
        .map(|r| {
            statements
                .get(r.line)
                .and_then(|line| line_index.get(&line).copied())
        })
        .collect();
    let mut line_writes = BTreeMap::<usize, Vec<usize>>::new();
    for (i, d) in defs.iter().enumerate() {
        if !d.alias_derived && facts.writes.contains_key(&(d.path.clone(), d.start_byte)) {
            line_writes.entry(d.line).or_default().push(i);
        }
    }
    let mut node_of = |line: usize| {
        statements
            .get(line)
            .and_then(|line| line_index.get(&line).copied())
    };
    let closure_nodes: Vec<_> = facts.closures.iter().map(|c| node_of(c.line)).collect();
    let call_nodes: Vec<Vec<_>> = facts
        .closures
        .iter()
        .map(|c| c.calls.iter().map(|call| node_of(call.line)).collect())
        .collect();
    let mut node_line = vec![0usize; successors.len()];
    for (&line, &n) in line_index {
        if n < node_line.len() {
            node_line[n] = line;
        }
    }
    let mut def_pos = vec![None; defs.len()];
    let (mut node_writes, mut node_sources) = (BTreeMap::new(), BTreeMap::new());
    for (i, d) in defs.iter().enumerate() {
        if d.alias_derived {
            continue;
        }
        if let Some(meta) = facts.write_meta.get(&(d.path.clone(), d.start_byte)) {
            def_pos[i] = Some(&meta.pos);
            if let Some(n) = mapped_defs[i] {
                node_writes.entry(n).or_insert_with(Vec::new).push(i);
                if d.path.has_fields() {
                    node_sources.entry(n).or_insert_with(Vec::new).push(i);
                }
            }
        }
    }
    let mut lane = Lane {
        defs,
        facts: &facts,
        gen: &gen,
        kill: &kill,
        successors,
        later_killed,
        line_writes,
        def_pos,
        node_writes,
        node_sources,
        node_line,
        closure_nodes,
        call_nodes,
        capture_live: BTreeMap::new(),
        deferred_live: BTreeMap::new(),
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
                read.receiver.clone(),
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
