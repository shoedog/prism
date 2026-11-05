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

    pub(super) fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub(super) fn node_set(&self) -> BTreeSet<NodeIndex> {
        self.nodes.iter().map(|(_, n)| *n).collect()
    }

    /// The lookup indexes without the hidden nodes.
    pub(super) fn view(
        &self,
        var_index: &BTreeMap<LegacyVarKey, NodeIndex>,
        exact_var_index: &ExactVarIndex,
    ) -> (BTreeMap<LegacyVarKey, NodeIndex>, ExactVarIndex) {
        let mut var_index = var_index.clone();
        let mut exact_var_index = exact_var_index.clone();
        for (key, idx) in &self.nodes {
            if var_index.get(key) == Some(idx) {
                var_index.remove(key);
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
        (var_index, exact_var_index)
    }
}
