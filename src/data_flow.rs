//! Data flow graph construction for intraprocedural and interprocedural analysis.
//!
//! Builds def-use chains within functions and tracks flow through function
//! arguments and return values. Used by chopping, taint analysis, circular
//! slice, and delta slice.

use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::cpg::{FlowConfidence, RdFileStats};
use crate::finding_confidence::EvidenceHop;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
mod build;
mod types;
pub(crate) use types::{ExactFlowEdge, ExactVarLocation};
pub use types::{FlowEdge, FlowPath, VarAccessKind, VarLocation};
#[cfg(any(test, feature = "dfg-oracle"))]
pub mod oracle;
#[cfg(any(test, feature = "dfg-oracle"))]
pub mod test_support;
#[cfg(test)]
mod tests;

/// The data flow graph for a set of parsed files.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataFlowGraph {
    /// All def-use edges.
    pub edges: Vec<FlowEdge>,
    /// Index: (file, function, function_start_line, access_path) → definitions
    pub defs: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>>,
    /// Index: (file, function, function_start_line, access_path) → uses
    pub uses: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>>,
    /// Forward adjacency: def location → use locations it reaches
    pub forward: BTreeMap<VarLocation, Vec<VarLocation>>,
    /// Backward adjacency: use location → def locations it comes from
    pub backward: BTreeMap<VarLocation, Vec<VarLocation>>,
    /// One label per unique `(from, to)` FlowEdge endpoint pair. Primary state
    /// with the same file-partitioned lifecycle as `edges`.
    pub labels: BTreeMap<(VarLocation, VarLocation), FlowConfidence>,
    /// Supplemental byte-distinct producer facts absent from the legacy exact
    /// endpoint population. Confidence is classified before legacy key reduction.
    pub(crate) exact_labels: BTreeMap<ExactFlowEdge, FlowConfidence>,
    /// Per-file reaching-definitions availability counters.
    pub rd_function_stats: BTreeMap<String, RdFileStats>,
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
    fn rebuild_adjacency(&mut self) {
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

    fn insert_label(
        labels: &mut BTreeMap<(VarLocation, VarLocation), FlowConfidence>,
        key: (VarLocation, VarLocation),
        label: FlowConfidence,
    ) {
        labels
            .entry(key)
            .and_modify(|stored| *stored = stored.worst(label))
            .or_insert(label);
    }

    /// Find all locations reachable forward from a given location (transitive).
    pub fn forward_reachable(&self, from: &VarLocation) -> BTreeSet<VarLocation> {
        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(from.clone());

        while let Some(loc) = queue.pop_front() {
            if !visited.insert(loc.clone()) {
                continue;
            }
            if let Some(nexts) = self.forward.get(&loc) {
                for next in nexts {
                    queue.push_back(next.clone());
                }
            }
            // Also follow through: if this is a use, find the def on the same line
            // (assignment propagation: x = y means use of y flows to def of x)
            if loc.kind == VarAccessKind::Use {
                let _key = (loc.file.clone(), loc.function.clone(), loc.path.clone());
                // Find defs of other variables on the same line
                for ((f, func, fsl, _path), def_locs) in &self.defs {
                    if f == &loc.file && func == &loc.function && *fsl == loc.function_start_line {
                        for dl in def_locs {
                            if dl.line == loc.line && !visited.contains(dl) {
                                queue.push_back(dl.clone());
                            }
                        }
                    }
                }
            }
        }

        visited.remove(from);
        visited
    }

    /// Find all locations reachable backward from a given location (transitive).
    pub fn backward_reachable(&self, from: &VarLocation) -> BTreeSet<VarLocation> {
        self.backward_reachable_labeled(from).0
    }

    /// Backward reachability plus the concrete labeled DataFlow hops selected by
    /// the walk. The reachable set is identical to [`Self::backward_reachable`].
    pub fn backward_reachable_labeled(
        &self,
        from: &VarLocation,
    ) -> (BTreeSet<VarLocation>, Vec<EvidenceHop>) {
        let mut visited = BTreeSet::new();
        let mut hops = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(from.clone());

        while let Some(loc) = queue.pop_front() {
            if !visited.insert(loc.clone()) {
                continue;
            }
            if let Some(prevs) = self.backward.get(&loc) {
                let mut prevs = prevs.clone();
                prevs.sort();
                prevs.dedup();
                for prev in &prevs {
                    if visited.contains(prev) {
                        continue;
                    }
                    if let Some(&confidence) = self.labels.get(&(prev.clone(), loc.clone())) {
                        hops.push(EvidenceHop::DataFlow {
                            from: prev.clone(),
                            to: loc.clone(),
                            confidence,
                        });
                    }
                    queue.push_back(prev.clone());
                }
            }
        }

        visited.remove(from);
        (visited, hops)
    }

    /// Find all statements on any data flow path between source and sink.
    pub fn chop(
        &self,
        source_file: &str,
        source_line: usize,
        sink_file: &str,
        sink_line: usize,
    ) -> BTreeSet<(String, usize)> {
        // Forward reachable from source
        let source_locs: Vec<VarLocation> = self.all_locations_at(source_file, source_line);
        let mut forward_reachable = BTreeSet::new();
        for loc in &source_locs {
            forward_reachable.extend(self.forward_reachable(loc));
            forward_reachable.insert(loc.clone());
        }

        // Backward reachable from sink
        let sink_locs: Vec<VarLocation> = self.all_locations_at(sink_file, sink_line);
        let mut backward_reachable = BTreeSet::new();
        for loc in &sink_locs {
            backward_reachable.extend(self.backward_reachable(loc));
            backward_reachable.insert(loc.clone());
        }

        // Intersection: statements on paths between source and sink
        let on_path: BTreeSet<(String, usize)> = forward_reachable
            .intersection(&backward_reachable)
            .map(|loc| (loc.file.clone(), loc.line))
            .collect();

        // Always include source and sink
        let mut result = on_path;
        result.insert((source_file.to_string(), source_line));
        result.insert((sink_file.to_string(), sink_line));
        result
    }

    /// Get all VarLocations at a specific file and line.
    pub(crate) fn all_locations_at(&self, file: &str, line: usize) -> Vec<VarLocation> {
        let mut result = Vec::new();
        for locs in self.defs.values() {
            for loc in locs {
                if loc.file == file && loc.line == line {
                    result.push(loc.clone());
                }
            }
        }
        for locs in self.uses.values() {
            for loc in locs {
                if loc.file == file && loc.line == line {
                    result.push(loc.clone());
                }
            }
        }
        result
    }

    /// Forward taint propagation from a set of tainted locations.
    pub fn taint_forward(&self, taint_sources: &[(String, usize)]) -> Vec<FlowPath> {
        let mut paths = Vec::new();

        for (file, line) in taint_sources {
            let source_locs = self.all_locations_at(file, *line);
            for source in &source_locs {
                let reachable = self.forward_reachable(source);
                if !reachable.is_empty() {
                    // Build a path from source to each reachable sink
                    let path = FlowPath {
                        edges: reachable
                            .iter()
                            .map(|target| FlowEdge {
                                from: source.clone(),
                                to: target.clone(),
                            })
                            .collect(),
                        cleansed_for: BTreeSet::new(),
                    };
                    paths.push(path);
                }
            }
        }

        paths
    }

    /// Get all definition locations of a variable by base name in a file.
    /// Matches any AccessPath whose base equals `var_name` (field-insensitive lookup).
    pub fn all_defs_of(&self, file: &str, var_name: &str) -> Vec<VarLocation> {
        let mut result = Vec::new();
        for ((f, _func, _fsl, path), locs) in &self.defs {
            if f == file && path.base == var_name {
                result.extend(locs.clone());
            }
        }
        result
    }
}
