//! Lane-local receiver Uses (#639 S2 fold 1/2).
//!
//! A receiver Use that exists only as the endpoint of a Go recall edge gets a
//! graph node (so taint/chop/leftflow walk the edge) and stays in the stored,
//! public lookup indexes (a cache load rebuilds those from every node). The
//! argument/return passes (Step 5b/5c) look only through a filtered view that
//! omits those nodes, so they never bind them to a callee. A key shared with an
//! ordinary Use is one node (Go merges same-line same-path Uses) and is not
//! hidden.
use super::*;

pub(super) struct HiddenUses {
    spans: std::collections::HashMap<String, std::collections::HashSet<(usize, usize)>>,
    keys: BTreeSet<LegacyVarKey>,
    nodes: Vec<(LegacyVarKey, NodeIndex)>,
}

impl HiddenUses {
    pub(super) fn new(dfg: &DataFlowGraph) -> Self {
        let spans = dfg.recall_only_use_spans();
        let is_recall = |loc: &VarLocation| {
            spans
                .get(loc.file.as_str())
                .is_some_and(|s| s.contains(&(loc.start_byte, loc.end_byte)))
        };
        let mut keys = BTreeSet::new();
        if !spans.is_empty() {
            for locs in dfg.uses().map(|(_, v)| v) {
                for loc in locs.iter().filter(|l| is_recall(l)) {
                    keys.insert(CodePropertyGraph::legacy_var_key(loc, VarAccess::Use));
                }
            }
            let ordinary: BTreeSet<_> = dfg
                .uses()
                .flat_map(|(_, v)| v.iter())
                .filter(|l| !is_recall(l))
                .map(|l| CodePropertyGraph::legacy_var_key(l, VarAccess::Use))
                .filter(|k| keys.contains(k))
                .collect();
            keys.retain(|k| !ordinary.contains(k));
        }
        Self {
            spans,
            keys,
            nodes: Vec::new(),
        }
    }

    /// Note the node `materialize_var_location` produced for `loc`, when that
    /// call created it.
    pub(super) fn note(&mut self, loc: &VarLocation, created: bool, idx: NodeIndex) {
        if created
            && self
                .spans
                .get(loc.file.as_str())
                .is_some_and(|s| s.contains(&(loc.start_byte, loc.end_byte)))
        {
            let key = CodePropertyGraph::legacy_var_key(loc, VarAccess::Use);
            if self.keys.contains(&key) {
                self.nodes.push((key, idx));
            }
        }
    }

    pub(super) fn node_set(&self) -> BTreeSet<NodeIndex> {
        self.nodes.iter().map(|(_, n)| *n).collect()
    }

    /// Hide the lane-local nodes from the lookup indexes in place, no copy.
    /// `exact_var_index` is a build-local that is not read after Step 5b, so
    /// its entries are dropped for good; the stored `var_index` entries are
    /// put back when the returned guard is dropped, on every path including a
    /// panic, so the public index stays complete.
    pub(super) fn detach<'a>(
        &self,
        var_index: &'a mut BTreeMap<LegacyVarKey, NodeIndex>,
        exact_var_index: &mut ExactVarIndex,
    ) -> Detached<'a> {
        let mut removed = Vec::new();
        for (key, idx) in &self.nodes {
            if var_index.get(key) == Some(idx) {
                if let Some(entry) = var_index.remove_entry(key) {
                    removed.push(entry);
                }
            }
            if let Some(spans) = exact_var_index.get_mut(key) {
                for nodes in spans.values_mut() {
                    nodes.retain(|n| n != idx);
                }
                spans.retain(|_, nodes| !nodes.is_empty());
                if spans.is_empty() {
                    exact_var_index.remove(key);
                }
            }
        }
        Detached { var_index, removed }
    }
}

/// `var_index` with the hidden entries removed; reinserts them on drop.
pub(super) struct Detached<'a> {
    var_index: &'a mut BTreeMap<LegacyVarKey, NodeIndex>,
    removed: Vec<(LegacyVarKey, NodeIndex)>,
}

impl std::ops::Deref for Detached<'_> {
    type Target = BTreeMap<LegacyVarKey, NodeIndex>;
    fn deref(&self) -> &Self::Target {
        self.var_index
    }
}

impl Drop for Detached<'_> {
    fn drop(&mut self) {
        for (key, idx) in self.removed.drain(..) {
            self.var_index.insert(key, idx);
        }
    }
}

/// Go receiver recall edges whose selector is a func-typed field of the
/// package are removed (or restored) against the whole repository here, so
/// cold, full-hit and partial-hit builds agree.
pub(super) fn refuse_func_fields(
    dfg: &mut DataFlowGraph,
    files: &BTreeMap<String, ParsedFile>,
    package_files: Option<&BTreeMap<String, ParsedFile>>,
) {
    if let Some(refuse) = crate::cpg::go_package_fields::refusals(dfg, files, package_files) {
        dfg.apply_receiver_refusals(&refuse);
    }
}

impl CodePropertyGraph {
    /// Step 4's Use nodes, noting the lane-local ones.
    pub(super) fn materialize_uses(
        dfg: &DataFlowGraph,
        files: &BTreeMap<String, ParsedFile>,
        graph: &mut DiGraph<CpgNode, CpgEdge>,
        var_index: &mut BTreeMap<LegacyVarKey, NodeIndex>,
        exact_var_index: &mut ExactVarIndex,
        location_index: &mut BTreeMap<(String, usize), Vec<NodeIndex>>,
    ) -> HiddenUses {
        let mut hidden = HiddenUses::new(dfg);
        for locs in dfg.uses().map(|(_, v)| v) {
            for loc in locs {
                let before = graph.node_count();
                let idx = Self::materialize_var_location(
                    loc,
                    VarAccess::Use,
                    files,
                    graph,
                    var_index,
                    exact_var_index,
                    location_index,
                );
                hidden.note(loc, graph.node_count() > before, idx);
            }
        }
        hidden
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(line: usize) -> LegacyVarKey {
        (
            "a.go".into(),
            "f".into(),
            1,
            line,
            AccessPath::simple("r"),
            VarAccess::Use,
        )
    }

    fn hidden(nodes: Vec<(LegacyVarKey, NodeIndex)>) -> HiddenUses {
        HiddenUses {
            spans: Default::default(),
            keys: nodes.iter().map(|(k, _)| k.clone()).collect(),
            nodes,
        }
    }

    fn exact_of(entries: &[(usize, usize, usize)]) -> ExactVarIndex {
        let mut e = ExactVarIndex::new();
        for &(line, n, span) in entries {
            e.entry(key(line))
                .or_default()
                .entry((span, span + 1))
                .or_default()
                .push(NodeIndex::new(n));
        }
        e
    }

    #[test]
    fn detach_hides_only_hidden_entries_and_drop_restores_them() {
        let (a, b, c) = (NodeIndex::new(0), NodeIndex::new(1), NodeIndex::new(2));
        let mut var_index = BTreeMap::from([(key(1), a), (key(2), b), (key(3), c)]);
        let original = var_index.clone();
        let mut exact = exact_of(&[(1, 0, 10), (2, 1, 20)]);
        // key(1) -> node a hidden; key(2) is *shared*: the entry holds the
        // visible node b while the hidden node c shares its key, so it stays.
        let h = hidden(vec![(key(1), a), (key(2), c)]);
        {
            let view = h.detach(&mut var_index, &mut exact);
            assert!(!view.contains_key(&key(1)), "hidden entry is detached");
            assert_eq!(view.get(&key(2)), Some(&b), "shared entry stays visible");
            assert_eq!(view.get(&key(3)), Some(&c));
        }
        assert_eq!(var_index, original, "stored var_index is complete again");
        assert!(
            !exact.contains_key(&key(1)),
            "exact entry of hidden node gone"
        );
        assert!(
            exact.contains_key(&key(2)),
            "exact entry of visible node kept"
        );
    }

    #[test]
    fn detach_removes_only_the_hidden_node_from_a_shared_exact_entry() {
        let (a, b) = (NodeIndex::new(0), NodeIndex::new(1));
        let mut var_index = BTreeMap::from([(key(1), b)]);
        let mut exact = exact_of(&[(1, 0, 10), (1, 1, 20)]);
        let h = hidden(vec![(key(1), a)]);
        drop(h.detach(&mut var_index, &mut exact));
        let spans = &exact[&key(1)];
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[&(20, 21)], vec![b]);
        assert_eq!(var_index[&key(1)], b);
    }

    #[test]
    fn detach_with_nothing_hidden_is_a_no_op() {
        let mut var_index = BTreeMap::from([(key(1), NodeIndex::new(0))]);
        let mut exact = exact_of(&[(1, 0, 10)]);
        let (v0, e0) = (var_index.clone(), exact.clone());
        drop(hidden(Vec::new()).detach(&mut var_index, &mut exact));
        assert_eq!((var_index, exact), (v0, e0));
    }

    #[test]
    fn var_index_is_restored_when_the_consumer_panics() {
        let a = NodeIndex::new(0);
        let mut var_index = BTreeMap::from([(key(1), a)]);
        let mut exact = ExactVarIndex::new();
        let h = hidden(vec![(key(1), a)]);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let view = h.detach(&mut var_index, &mut exact);
            assert!(view.is_empty());
            panic!("consumer failed between detach and restore");
        }));
        assert!(result.is_err());
        assert_eq!(var_index.get(&key(1)), Some(&a), "restored on unwind");
    }
}
