//! Legacy traversal bodies over ordered borrowed views.
use super::*;
use crate::finding_confidence::EvidenceHop;
use std::collections::VecDeque;

impl DataFlowGraph {
    /// Find all locations reachable forward from a given location (transitive).
    pub fn forward_reachable(&self, from: &VarLocation) -> BTreeSet<VarLocation> {
        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(from.clone());

        while let Some(loc) = queue.pop_front() {
            if !visited.insert(loc.clone()) {
                continue;
            }
            if let Some(nexts) = self.successors(&loc) {
                for next in nexts {
                    queue.push_back(next.clone());
                }
            }
            // Also follow through: if this is a use, find the def on the same line
            // (assignment propagation: x = y means use of y flows to def of x)
            if loc.kind == VarAccessKind::Use {
                let _key = (loc.file.clone(), loc.function.clone(), loc.path.clone());
                // Find defs of other variables on the same line
                for ((f, func, fsl, _path), def_locs) in self.defs() {
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
            if let Some(prevs) = self.predecessors(&loc) {
                let mut prevs = prevs.to_vec();
                prevs.sort();
                prevs.dedup();
                for prev in &prevs {
                    if visited.contains(prev) {
                        continue;
                    }
                    if let Some(confidence) = self.label(prev, &loc) {
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
        for (_, locs) in self.defs() {
            for loc in locs {
                if loc.file == file && loc.line == line {
                    result.push(loc.clone());
                }
            }
        }
        for (_, locs) in self.uses() {
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
        for ((f, _func, _fsl, path), locs) in self.defs() {
            if f == file && path.base == var_name {
                result.extend(locs.iter().cloned());
            }
        }
        result
    }
}
