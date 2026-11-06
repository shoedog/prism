//! Compact `var_index`: the CPG's variable lookup index (#437 memory slice 3).
//!
//! The index used to be a `BTreeMap<(file, function, fn_start_line, line,
//! AccessPath, VarAccess), NodeIndex>` whose every key was a copy of fields the
//! `CpgNode::Variable` node already holds. `VarIndex` stores only the
//! `NodeIndex` (4 bytes) in a `hashbrown::HashTable`; the hash and the equality
//! test are derived from `graph[idx]`, so every operation takes the graph the
//! indexed nodes live in. Nodes are never mutated after they are indexed.
//!
//! Semantics preserved from the map: first-wins on duplicate keys, and the
//! `HiddenUses` rule (a hidden node's entry answers `None`, a later duplicate
//! never takes its place) is a mask (`hide`/`unhide`) rather than a removal, so
//! nothing needs re-inserting. Iteration order is not defined; the one ordered
//! consumer (`all_defs_of`) sorts its matches with `sort_by_key_order`.

use std::collections::BTreeSet;
use std::hash::{BuildHasher, Hash, Hasher};

use hashbrown::{DefaultHashBuilder, HashTable};
use petgraph::graph::{DiGraph, NodeIndex};

use super::{CpgEdge, CpgNode, VarAccess};
use crate::access_path::AccessPath;

type Graph = DiGraph<CpgNode, CpgEdge>;

/// The owned key shape the map used; kept for callers that already build one.
pub(crate) type LegacyVarKey = (String, String, usize, usize, AccessPath, VarAccess);

/// A borrowed lookup key.
#[derive(Clone, Copy)]
pub(crate) struct VarKeyRef<'a> {
    pub file: &'a str,
    pub function: &'a str,
    pub function_start_line: usize,
    pub line: usize,
    pub path: &'a AccessPath,
    pub access: VarAccess,
}

impl<'a> VarKeyRef<'a> {
    pub(crate) fn from_legacy(key: &'a LegacyVarKey) -> Self {
        VarKeyRef {
            file: &key.0,
            function: &key.1,
            function_start_line: key.2,
            line: key.3,
            path: &key.4,
            access: key.5,
        }
    }

    /// The key of a Variable node; `None` for any other node kind.
    fn of_node(node: &'a CpgNode) -> Option<Self> {
        match node {
            CpgNode::Variable {
                path,
                file,
                function,
                function_start_line,
                line,
                access,
                ..
            } => Some(VarKeyRef {
                file,
                function,
                function_start_line: *function_start_line,
                line: *line,
                path,
                access: *access,
            }),
            _ => None,
        }
    }

    #[cfg(test)]
    fn to_legacy(self) -> LegacyVarKey {
        (
            self.file.to_string(),
            self.function.to_string(),
            self.function_start_line,
            self.line,
            self.path.clone(),
            self.access,
        )
    }

    fn matches(&self, other: &VarKeyRef<'_>) -> bool {
        self.function_start_line == other.function_start_line
            && self.line == other.line
            && self.access == other.access
            && self.file == other.file
            && self.function == other.function
            && self.path == other.path
    }
}

fn hash_key(hasher: &DefaultHashBuilder, key: &VarKeyRef<'_>) -> u64 {
    let mut h = hasher.build_hasher();
    key.file.hash(&mut h);
    key.function.hash(&mut h);
    key.function_start_line.hash(&mut h);
    key.line.hash(&mut h);
    key.path.hash(&mut h);
    key.access.hash(&mut h);
    h.finish()
}

#[derive(Default)]
pub(crate) struct VarIndex {
    table: HashTable<NodeIndex>,
    hasher: DefaultHashBuilder,
    /// Nodes whose entries are masked from `get`/`get_ref` (see `hide`). Empty
    /// outside the Step 5b/5c window of an assembly.
    hidden: BTreeSet<NodeIndex>,
    /// Test seam: when set, lookups answer from this map alone (tests build
    /// states whose keys deliberately disagree with the node they point at).
    #[cfg(test)]
    overlay: Option<std::collections::BTreeMap<LegacyVarKey, NodeIndex>>,
}

impl Clone for VarIndex {
    fn clone(&self) -> Self {
        VarIndex {
            table: self.table.clone(),
            hasher: self.hasher,
            hidden: self.hidden.clone(),
            #[cfg(test)]
            overlay: self.overlay.clone(),
        }
    }
}

impl std::fmt::Debug for VarIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VarIndex({} entries)", self.table.len())
    }
}

fn key_of(graph: &Graph, idx: NodeIndex) -> VarKeyRef<'_> {
    graph
        .node_weight(idx)
        .and_then(VarKeyRef::of_node)
        .expect("var_index entries are Variable nodes of the graph they were built with")
}

impl VarIndex {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.table.len()
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn clear(&mut self) {
        self.table.clear();
    }

    /// Index every Variable node of `graph` in node order, first wins (the
    /// rebuild a cache load performs).
    #[cfg(test)]
    pub(crate) fn from_graph(graph: &Graph) -> Self {
        let mut index = Self::new();
        for idx in graph.node_indices() {
            if matches!(graph[idx], CpgNode::Variable { .. }) {
                index.insert_first_wins(graph, idx);
            }
        }
        index
    }

    pub(crate) fn get_ref(&self, graph: &Graph, key: VarKeyRef<'_>) -> Option<NodeIndex> {
        #[cfg(test)]
        if let Some(overlay) = &self.overlay {
            return overlay
                .get(&key.to_legacy())
                .copied()
                .filter(|n| !self.hidden.contains(n));
        }
        let hash = hash_key(&self.hasher, &key);
        self.table
            .find(hash, |&n| key_of(graph, n).matches(&key))
            .copied()
            .filter(|n| self.hidden.is_empty() || !self.hidden.contains(n))
    }

    /// Mask these nodes' entries from lookups until `unhide`. Nothing is
    /// removed from the table, so there is nothing to restore; a masked key
    /// answers `None` exactly as a removed entry did (a later duplicate node
    /// never takes its place).
    pub(crate) fn hide(&mut self, nodes: impl IntoIterator<Item = NodeIndex>) {
        self.hidden.extend(nodes);
    }

    pub(crate) fn unhide(&mut self) {
        self.hidden.clear();
    }

    /// Lookup by the owned tuple key.
    pub(crate) fn get(&self, graph: &Graph, key: &LegacyVarKey) -> Option<NodeIndex> {
        self.get_ref(graph, VarKeyRef::from_legacy(key))
    }

    /// `or_insert`: index `idx` under its own key unless the key is taken.
    /// Returns the node the key now points at.
    pub(crate) fn insert_first_wins(&mut self, graph: &Graph, idx: NodeIndex) -> NodeIndex {
        let key = key_of(graph, idx);
        let hash = hash_key(&self.hasher, &key);
        if let Some(&existing) = self.table.find(hash, |&n| key_of(graph, n).matches(&key)) {
            return existing;
        }
        let hasher = self.hasher;
        self.table
            .insert_unique(hash, idx, |&n| hash_key(&hasher, &key_of(graph, n)));
        idx
    }

    /// Every entry as the old map held it (test comparison and fixtures).
    #[cfg(test)]
    pub(crate) fn to_map(
        &self,
        graph: &Graph,
    ) -> std::collections::BTreeMap<LegacyVarKey, NodeIndex> {
        if let Some(overlay) = &self.overlay {
            return overlay.clone();
        }
        self.nodes()
            .map(|n| (key_of(graph, n).to_legacy(), n))
            .collect()
    }

    /// An index that answers from `map` alone, whatever the graph holds.
    #[cfg(test)]
    pub(crate) fn from_map(map: std::collections::BTreeMap<LegacyVarKey, NodeIndex>) -> Self {
        VarIndex {
            overlay: Some(map),
            ..Self::default()
        }
    }

    pub(crate) fn nodes(&self) -> impl Iterator<Item = NodeIndex> + '_ {
        self.table.iter().copied()
    }

    /// Nodes in the old map's key order.
    #[cfg(test)]
    pub(crate) fn sorted_nodes(&self, graph: &Graph) -> Vec<NodeIndex> {
        let mut nodes: Vec<NodeIndex> = self.nodes().collect();
        self.sort_by_key_order(graph, &mut nodes);
        nodes
    }

    /// Sort `nodes` into the old map's key order (file, function, function
    /// start line, line, path, access).
    pub(crate) fn sort_by_key_order(&self, graph: &Graph, nodes: &mut [NodeIndex]) {
        nodes.sort_by(|&a, &b| {
            let (ka, kb) = (key_of(graph, a), key_of(graph, b));
            (
                ka.file,
                ka.function,
                ka.function_start_line,
                ka.line,
                ka.path,
                ka.access,
            )
                .cmp(&(
                    kb.file,
                    kb.function,
                    kb.function_start_line,
                    kb.line,
                    kb.path,
                    kb.access,
                ))
        });
    }
}
