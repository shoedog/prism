//! Control-flow-graph query methods on `CodePropertyGraph` (Phase 6).

use crate::data_flow::VarLocation;

use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use std::collections::{BTreeMap, BTreeSet};

use super::{CodePropertyGraph, CpgEdge, CpgNode};

impl CodePropertyGraph {
    // -----------------------------------------------------------------------
    // CFG queries (Phase 6)
    // -----------------------------------------------------------------------

    /// Check if control flow edges are present in this CPG.
    pub fn has_cfg_edges(&self) -> bool {
        self.graph
            .edge_indices()
            .any(|e| matches!(self.graph[e], CpgEdge::ControlFlow))
    }

    /// Get all CFG successors of a node (following only ControlFlow edges).
    pub fn cfg_successors(&self, idx: NodeIndex) -> Vec<NodeIndex> {
        self.graph
            .edges(idx)
            .filter(|e| matches!(e.weight(), CpgEdge::ControlFlow))
            .map(|e| e.target())
            .collect()
    }

    /// Get all CFG predecessors of a node (following only ControlFlow edges).
    pub fn cfg_predecessors(&self, idx: NodeIndex) -> Vec<NodeIndex> {
        self.graph
            .edges_directed(idx, petgraph::Direction::Incoming)
            .filter(|e| matches!(e.weight(), CpgEdge::ControlFlow))
            .map(|e| e.source())
            .collect()
    }

    /// Get the Statement node at a given file and line, if one exists.
    pub fn statement_at(&self, file: &str, line: usize) -> Option<NodeIndex> {
        let key = (file.to_string(), line);
        self.location_index.get(&key).and_then(|nodes| {
            nodes
                .iter()
                .find(|&&idx| matches!(self.graph[idx], CpgNode::Statement { .. }))
                .copied()
        })
    }

    /// Check if `line` is CFG-reachable from the given source, handling
    /// multi-line statement continuation lines.
    ///
    /// `cfg_set` contains the statement-start lines that are CFG-reachable.
    /// If `line` has no statement starting exactly there (it's a continuation
    /// of a multi-line statement), this walks back up to 20 lines to find the
    /// enclosing statement start and checks if THAT statement is reachable.
    ///
    /// Example: in `cert = lib->creds->create(...,\n  BUILD_FROM_FILE, str, ...)`,
    /// `BUILD_FROM_FILE` is on the continuation line. The statement starts one line
    /// above. If the statement start is reachable, so is the continuation.
    fn cfg_reachable_including_continuation(
        &self,
        file: &str,
        line: usize,
        cfg_set: &BTreeSet<(String, usize)>,
    ) -> bool {
        if cfg_set.contains(&(file.to_string(), line)) {
            return true;
        }
        // Only look back if there is no statement starting at this exact line.
        // A continuation line has no statement node; a new statement does.
        if self.statement_at(file, line).is_none() {
            const MAX_STMT_SPAN: usize = 20;
            for offset in 1..=MAX_STMT_SPAN.min(line.saturating_sub(1)) {
                let check_line = line - offset;
                if self.statement_at(file, check_line).is_some() {
                    // Found the enclosing statement start — check if it's reachable.
                    return cfg_set.contains(&(file.to_string(), check_line));
                }
            }
        }
        false
    }

    /// Count ControlFlow edges in the graph.
    pub fn cfg_edge_count(&self) -> usize {
        self.graph
            .edge_indices()
            .filter(|&e| matches!(self.graph[e], CpgEdge::ControlFlow))
            .count()
    }

    // -----------------------------------------------------------------------
    // CFG-constrained analysis (Phase 6 PR C)
    // -----------------------------------------------------------------------

    /// Collect all `(file, line)` pairs CFG-reachable from a given line.
    ///
    /// Uses BFS over ControlFlow edges from the Statement node at the given
    /// location. Returns the set of reachable `(file, line)` pairs (excluding
    /// the start). Returns an empty set if no Statement node exists at the
    /// start location or if the CPG has no CFG edges.
    pub fn cfg_reachable_lines(&self, file: &str, line: usize) -> BTreeSet<(String, usize)> {
        let start = match self.statement_at(file, line) {
            Some(idx) => idx,
            None => return BTreeSet::new(),
        };

        let reachable = self.reachable_forward(start, &|e| matches!(e, CpgEdge::ControlFlow));

        reachable
            .into_iter()
            .map(|idx| {
                let node = &self.graph[idx];
                (node.file().to_string(), node.line())
            })
            .collect()
    }

    /// CFG-constrained forward taint propagation.
    ///
    /// Like `taint_forward()`, but filters out DFG-reachable nodes that are not
    /// also CFG-reachable from the taint source. This prunes taint paths through
    /// dead code (after return/break) and guarded branches.
    ///
    /// Falls back to pure DFG taint when no CFG edges are present.
    pub fn taint_forward_cfg(
        &self,
        taint_sources: &[(String, usize)],
    ) -> Vec<crate::data_flow::FlowPath> {
        if !self.has_cfg_edges() {
            return self.taint_forward(taint_sources);
        }

        // Build per-source CFG reachability sets
        let mut cfg_reachable: BTreeMap<(String, usize), BTreeSet<(String, usize)>> =
            BTreeMap::new();
        for (file, line) in taint_sources {
            let key = (file.clone(), *line);
            if !cfg_reachable.contains_key(&key) {
                cfg_reachable.insert(key.clone(), self.cfg_reachable_lines(file, *line));
            }
        }

        let mut paths = Vec::new();

        for (file, line) in taint_sources {
            let source_nodes = self.nodes_at(file, *line);
            let cfg_set = cfg_reachable
                .get(&(file.clone(), *line))
                .cloned()
                .unwrap_or_default();

            for &src_idx in &source_nodes {
                if !matches!(self.graph[src_idx], CpgNode::Variable { .. }) {
                    continue;
                }
                let src_loc = match self.to_var_location(src_idx) {
                    Some(loc) => loc,
                    None => continue,
                };
                let reachable = self.dfg_forward_reachable(&src_loc);

                // Filter: keep only DFG-reachable targets that are also CFG-reachable.
                // Interprocedural targets (different file or function) bypass the CFG
                // filter since CFG edges are intraprocedural.
                let filtered: BTreeSet<VarLocation> = reachable
                    .into_iter()
                    .filter(|target| {
                        // Same line as source is always included
                        (target.file == *file && target.line == *line)
                            // Cross-function targets bypass CFG filter
                            || target.file != *file
                            || target.function != src_loc.function
                            // Intraprocedural: must be CFG-reachable.
                            // Also accepts continuation lines of multi-line statements
                            // (e.g. BUILD_FROM_FILE on line 256 when the call_expression
                            // starts at line 255 — line 255 is in cfg_set but 256 is not).
                            || self.cfg_reachable_including_continuation(
                                &target.file,
                                target.line,
                                &cfg_set,
                            )
                    })
                    .collect();

                if !filtered.is_empty() {
                    let path = crate::data_flow::FlowPath {
                        edges: filtered
                            .iter()
                            .map(|target| crate::data_flow::FlowEdge {
                                from: src_loc.clone(),
                                to: target.clone(),
                            })
                            .collect(),
                        cleansed_for: std::collections::BTreeSet::new(),
                    };
                    paths.push(path);
                }
            }
        }

        paths
    }

    /// CFG-constrained chop: find statements on data flow paths between source
    /// and sink that are also control-flow reachable.
    ///
    /// This intersects the DFG chop result with CFG reachability from the source
    /// and CFG backward-reachability from the sink, pruning data flow paths that
    /// pass through control-flow-unreachable code.
    ///
    /// Falls back to pure DFG chop when no CFG edges are present.
    pub fn dfg_cfg_chop(
        &self,
        source_file: &str,
        source_line: usize,
        sink_file: &str,
        sink_line: usize,
    ) -> BTreeSet<(String, usize)> {
        let dfg_result = self.dfg_chop(source_file, source_line, sink_file, sink_line);

        if !self.has_cfg_edges() {
            return dfg_result;
        }

        // CFG forward reachability from source
        let cfg_forward = {
            let mut set = self.cfg_reachable_lines(source_file, source_line);
            set.insert((source_file.to_string(), source_line));
            set
        };

        // CFG backward reachability from sink
        let cfg_backward = {
            let sink_stmt = self.statement_at(sink_file, sink_line);
            let mut set: BTreeSet<(String, usize)> = match sink_stmt {
                Some(idx) => self
                    .reachable_backward(idx, &|e| matches!(e, CpgEdge::ControlFlow))
                    .into_iter()
                    .map(|idx| {
                        let node = &self.graph[idx];
                        (node.file().to_string(), node.line())
                    })
                    .collect(),
                None => BTreeSet::new(),
            };
            set.insert((sink_file.to_string(), sink_line));
            set
        };

        // Intersect: DFG path ∩ CFG-forward-from-source ∩ CFG-backward-from-sink
        dfg_result
            .into_iter()
            .filter(|loc| cfg_forward.contains(loc) && cfg_backward.contains(loc))
            .collect()
    }
}
