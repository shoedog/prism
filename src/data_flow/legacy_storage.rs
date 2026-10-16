//! Transitional legacy storage and lifecycle; serialized field order is unchanged.
use super::*;

/// The data flow graph for a set of parsed files.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataFlowGraph {
    /// All def-use edges.
    pub(super) edges: Vec<FlowEdge>,
    /// Index: (file, function, function_start_line, access_path) → definitions
    pub(super) defs: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>>,
    /// Index: (file, function, function_start_line, access_path) → uses
    pub(super) uses: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>>,
    /// Forward adjacency: def location → use locations it reaches
    pub(super) forward: BTreeMap<VarLocation, Vec<VarLocation>>,
    /// Backward adjacency: use location → def locations it comes from
    pub(super) backward: BTreeMap<VarLocation, Vec<VarLocation>>,
    /// One label per unique `(from, to)` FlowEdge endpoint pair. Primary state
    /// with the same file-partitioned lifecycle as `edges`.
    pub(super) labels: BTreeMap<(VarLocation, VarLocation), FlowConfidence>,
    /// Supplemental byte-distinct producer facts absent from the legacy exact
    /// endpoint population. Confidence is classified before legacy key reduction.
    pub(super) exact_labels: BTreeMap<ExactFlowEdge, FlowConfidence>,
    /// Per-file reaching-definitions availability counters.
    pub(super) rd_function_stats: BTreeMap<String, RdFileStats>,
}

impl DataFlowGraph {
    /// Create an empty data flow graph with no edges.
    pub fn empty() -> Self {
        DataFlowGraph {
            edges: Vec::new(),
            defs: BTreeMap::new(),
            uses: BTreeMap::new(),
            forward: BTreeMap::new(),
            backward: BTreeMap::new(),
            labels: BTreeMap::new(),
            exact_labels: BTreeMap::new(),
            rd_function_stats: BTreeMap::new(),
        }
    }

    /// Remove all entries originating from the given files.
    ///
    /// Strips out edges, labels, defs, uses, and RD stats where the file matches
    /// any in `exclude`, then rebuilds adjacency from the retained edges.
    pub fn remove_files(&mut self, exclude: &BTreeSet<String>) {
        // Remove edges that involve excluded files (either end).
        self.edges
            .retain(|e| !exclude.contains(&e.from.file) && !exclude.contains(&e.to.file));

        // Remove defs/uses entries for excluded files.
        self.defs
            .retain(|(file, _, _, _), _| !exclude.contains(file));
        self.uses
            .retain(|(file, _, _, _), _| !exclude.contains(file));
        self.labels
            .retain(|(from, to), _| !exclude.contains(&from.file) && !exclude.contains(&to.file));
        self.exact_labels.retain(|edge, _| {
            !exclude.contains(&edge.from.file) && !exclude.contains(&edge.to.file)
        });
        self.rd_function_stats
            .retain(|file, _| !exclude.contains(file));

        // Rebuild adjacency from retained edges.
        self.rebuild_adjacency();
    }

    /// Merge another DataFlowGraph into this one.
    ///
    /// Adds all primary per-file state from `other`, then rebuilds adjacency.
    pub fn merge(&mut self, other: DataFlowGraph) {
        self.edges.extend(other.edges);
        for (key, locs) in other.defs {
            self.defs.entry(key).or_default().extend(locs);
        }
        for (key, locs) in other.uses {
            self.uses.entry(key).or_default().extend(locs);
        }
        for (key, label) in other.labels {
            Self::insert_label(&mut self.labels, key, label);
        }
        for (edge, label) in other.exact_labels {
            self.exact_labels
                .entry(edge)
                .and_modify(|stored| *stored = stored.worst(label))
                .or_insert(label);
        }
        for (file, stats) in other.rd_function_stats {
            self.rd_function_stats.entry(file).or_default().merge(stats);
        }
        self.rebuild_adjacency();
    }

    /// Rebuild the forward/backward adjacency maps from the current edges.
    pub(super) fn rebuild_adjacency(&mut self) {
        self.forward.clear();
        self.backward.clear();
        for edge in &self.edges {
            self.forward
                .entry(edge.from.clone())
                .or_default()
                .push(edge.to.clone());
            self.backward
                .entry(edge.to.clone())
                .or_default()
                .push(edge.from.clone());
        }
    }

    pub(super) fn insert_label(
        labels: &mut BTreeMap<(VarLocation, VarLocation), FlowConfidence>,
        key: (VarLocation, VarLocation),
        label: FlowConfidence,
    ) {
        labels
            .entry(key)
            .and_modify(|stored| *stored = stored.worst(label))
            .or_insert(label);
    }
}
