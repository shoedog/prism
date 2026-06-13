//! Query, traversal, data-flow, and type-resolution methods on
//! `CodePropertyGraph`.

use crate::access_path::AccessPath;
use crate::call_graph::FunctionId;
use crate::data_flow::{VarAccessKind, VarLocation};

use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::{CodePropertyGraph, CpgEdge, CpgNode, VarAccess};

impl CodePropertyGraph {
    // -----------------------------------------------------------------------
    // Index lookups
    // -----------------------------------------------------------------------

    /// Get the node index for a function by file and name.
    pub fn function_node(&self, file: &str, name: &str) -> Option<NodeIndex> {
        self.name_index
            .get(&(file.to_string(), name.to_string()))
            .and_then(|v| v.first().copied())
    }

    /// Get all candidate node indices for a function by file and name.
    pub fn function_candidates(&self, file: &str, name: &str) -> Vec<NodeIndex> {
        self.name_index
            .get(&(file.to_string(), name.to_string()))
            .cloned()
            .unwrap_or_default()
    }

    /// Get all node indices at a specific file and line.
    pub fn nodes_at(&self, file: &str, line: usize) -> Vec<NodeIndex> {
        self.location_index
            .get(&(file.to_string(), line))
            .cloned()
            .unwrap_or_default()
    }

    /// Get a node by its index.
    pub fn node(&self, idx: NodeIndex) -> &CpgNode {
        &self.graph[idx]
    }

    pub fn node_indices(&self) -> impl Iterator<Item = petgraph::graph::NodeIndex> + '_ {
        self.graph.node_indices()
    }

    /// Insertion-ordered (source, target, kind) dump for parity tests.
    pub fn edge_dump(&self) -> Vec<String> {
        self.graph
            .edge_references()
            .map(|e| {
                format!(
                    "{:?}->{:?}:{:?}",
                    e.source().index(),
                    e.target().index(),
                    e.weight()
                )
            })
            .collect()
    }

    /// Get the node index for a variable by its location.
    pub fn var_node(
        &self,
        file: &str,
        function: &str,
        function_start_line: usize,
        line: usize,
        path: &AccessPath,
        access: VarAccess,
    ) -> Option<NodeIndex> {
        self.var_index
            .get(&(
                file.to_string(),
                function.to_string(),
                function_start_line,
                line,
                path.clone(),
                access,
            ))
            .copied()
    }

    /// Total number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Total number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Get all function node indices.
    pub fn function_nodes(&self) -> Vec<NodeIndex> {
        self.func_index.values().copied().collect()
    }

    // -----------------------------------------------------------------------
    // Edge-filtered traversals
    // -----------------------------------------------------------------------

    /// Forward reachability following only edges that match the filter.
    ///
    /// Returns all nodes reachable from `start` by traversing edges where
    /// `edge_filter` returns true.
    pub fn reachable_forward(
        &self,
        start: NodeIndex,
        edge_filter: &dyn Fn(&CpgEdge) -> bool,
    ) -> BTreeSet<NodeIndex> {
        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(start);

        while let Some(node) = queue.pop_front() {
            if !visited.insert(node) {
                continue;
            }
            for edge in self.graph.edges(node) {
                if edge_filter(edge.weight()) && !visited.contains(&edge.target()) {
                    queue.push_back(edge.target());
                }
            }
        }

        visited.remove(&start);
        visited
    }

    /// Backward reachability following only edges whose reverse matches the filter.
    ///
    /// Uses petgraph's `edges_directed(Incoming)` to walk backward.
    pub fn reachable_backward(
        &self,
        start: NodeIndex,
        edge_filter: &dyn Fn(&CpgEdge) -> bool,
    ) -> BTreeSet<NodeIndex> {
        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(start);

        while let Some(node) = queue.pop_front() {
            if !visited.insert(node) {
                continue;
            }
            for edge in self
                .graph
                .edges_directed(node, petgraph::Direction::Incoming)
            {
                if edge_filter(edge.weight()) && !visited.contains(&edge.source()) {
                    queue.push_back(edge.source());
                }
            }
        }

        visited.remove(&start);
        visited
    }

    /// Check if there's a path from `source` to `target` following filtered edges.
    pub fn has_path(
        &self,
        source: NodeIndex,
        target: NodeIndex,
        edge_filter: &dyn Fn(&CpgEdge) -> bool,
    ) -> bool {
        if source == target {
            return true;
        }
        self.reachable_forward(source, edge_filter)
            .contains(&target)
    }

    // -----------------------------------------------------------------------
    // SCC — Strongly Connected Components via petgraph's Tarjan
    // -----------------------------------------------------------------------

    /// Find all strongly connected components in the subgraph defined by the
    /// edge filter. Returns only non-trivial SCCs (size >= 2).
    ///
    /// Uses petgraph's `tarjan_scc` on an edge-filtered view of the graph.
    pub fn strongly_connected_components(
        &self,
        edge_filter: &dyn Fn(&CpgEdge) -> bool,
    ) -> Vec<Vec<NodeIndex>> {
        // Build a filtered subgraph with only matching edges
        let filtered =
            petgraph::visit::EdgeFiltered::from_fn(&self.graph, |e| edge_filter(e.weight()));
        let sccs = petgraph::algo::tarjan_scc(&filtered);

        // Return only non-trivial SCCs (cycles)
        sccs.into_iter().filter(|scc| scc.len() >= 2).collect()
    }

    /// Find SCCs in the call graph (Call edges only).
    /// Returns cycles as lists of function node indices.
    pub fn call_graph_cycles(&self) -> Vec<Vec<NodeIndex>> {
        self.strongly_connected_components(&|e| matches!(e, CpgEdge::Call))
    }

    /// Find SCCs in the data flow graph (DataFlow edges only).
    pub fn data_flow_cycles(&self) -> Vec<Vec<NodeIndex>> {
        self.strongly_connected_components(&|e| matches!(e, CpgEdge::DataFlow))
    }

    // -----------------------------------------------------------------------
    // Hop-distance BFS (for gradient scoring)
    // -----------------------------------------------------------------------

    /// BFS with hop tracking. Returns (node_index, hop_distance) for all
    /// reachable nodes within `max_hops`, following filtered edges.
    pub fn bfs_with_distance(
        &self,
        starts: &[NodeIndex],
        max_hops: usize,
        edge_filter: &dyn Fn(&CpgEdge) -> bool,
    ) -> BTreeMap<NodeIndex, usize> {
        let mut distances: BTreeMap<NodeIndex, usize> = BTreeMap::new();
        let mut queue: VecDeque<(NodeIndex, usize)> = VecDeque::new();

        for &start in starts {
            distances.insert(start, 0);
            queue.push_back((start, 0));
        }

        while let Some((node, hop)) = queue.pop_front() {
            if hop >= max_hops {
                continue;
            }
            let next_hop = hop + 1;

            for edge in self.graph.edges(node) {
                if edge_filter(edge.weight()) {
                    let target = edge.target();
                    if !distances.contains_key(&target) || distances[&target] > next_hop {
                        distances.insert(target, next_hop);
                        queue.push_back((target, next_hop));
                    }
                }
            }
        }

        distances
    }

    // -----------------------------------------------------------------------
    // Chop: intersection of forward and backward reachability
    // -----------------------------------------------------------------------

    /// Find all nodes on any path from source to sink, following filtered edges.
    pub fn chop(
        &self,
        sources: &[NodeIndex],
        sinks: &[NodeIndex],
        edge_filter: &dyn Fn(&CpgEdge) -> bool,
    ) -> BTreeSet<NodeIndex> {
        let mut forward_set = BTreeSet::new();
        for &src in sources {
            forward_set.extend(self.reachable_forward(src, edge_filter));
            forward_set.insert(src);
        }

        let mut backward_set = BTreeSet::new();
        for &sink in sinks {
            backward_set.extend(self.reachable_backward(sink, edge_filter));
            backward_set.insert(sink);
        }

        forward_set.intersection(&backward_set).copied().collect()
    }

    // -----------------------------------------------------------------------
    // Bridge to existing types
    // -----------------------------------------------------------------------

    /// Convert a CPG Variable node back to a VarLocation for backward compatibility.
    pub fn to_var_location(&self, idx: NodeIndex) -> Option<VarLocation> {
        match &self.graph[idx] {
            CpgNode::Variable {
                path,
                file,
                function,
                function_start_line,
                line,
                access,
                start_byte,
                end_byte,
            } => Some(VarLocation {
                file: file.clone(),
                function: function.clone(),
                function_start_line: *function_start_line,
                line: *line,
                path: path.clone(),
                start_byte: *start_byte,
                end_byte: *end_byte,
                kind: match access {
                    VarAccess::Def => VarAccessKind::Def,
                    VarAccess::Use => VarAccessKind::Use,
                },
            }),
            _ => None,
        }
    }

    /// Convert a CPG Function node back to a FunctionId for backward compatibility.
    pub fn to_function_id(&self, idx: NodeIndex) -> Option<FunctionId> {
        match &self.graph[idx] {
            CpgNode::Function {
                name,
                file,
                start_line,
                end_line,
                ..
            } => Some(FunctionId {
                file: file.clone(),
                name: name.clone(),
                start_line: *start_line,
                end_line: *end_line,
            }),
            _ => None,
        }
    }

    /// Get all function nodes reachable from the given functions via Call edges.
    /// Returns (NodeIndex, FunctionId) pairs for convenience.
    pub fn call_reachable_functions(
        &self,
        start_func_names: &[(&str, &str)], // (file, name) pairs
    ) -> Vec<(NodeIndex, FunctionId)> {
        let mut result = Vec::new();
        let starts: Vec<NodeIndex> = start_func_names
            .iter()
            .filter_map(|(file, name)| self.function_node(file, name))
            .collect();

        for &start in &starts {
            let reachable = self.reachable_forward(start, &|e| matches!(e, CpgEdge::Call));
            for idx in reachable {
                if let Some(fid) = self.to_function_id(idx) {
                    result.push((idx, fid));
                }
            }
        }

        result.sort_by(|a, b| a.1.cmp(&b.1));
        result.dedup_by(|a, b| a.1 == b.1);
        result
    }

    // -----------------------------------------------------------------------
    // CallGraph-equivalent methods
    // -----------------------------------------------------------------------

    /// Find the function containing a specific line in a file.
    /// Equivalent to `CallGraph::function_at()`.
    pub fn function_at(&self, file: &str, line: usize) -> Option<(NodeIndex, FunctionId)> {
        for (&(ref f, ref _name, ref _sl), &idx) in &self.func_index {
            if f == file {
                if let CpgNode::Function {
                    start_line,
                    end_line,
                    ..
                } = self.graph[idx]
                {
                    if line >= start_line && line <= end_line {
                        return Some((idx, self.to_function_id(idx).unwrap()));
                    }
                }
            }
        }
        None
    }

    /// Find all callers of a function by name, up to a given depth.
    /// Returns (FunctionId, depth) pairs. Equivalent to `CallGraph::callers_of()`.
    pub fn callers_of(&self, func_name: &str, max_depth: usize) -> Vec<(FunctionId, usize)> {
        let mut result = Vec::new();
        let mut visited: BTreeSet<NodeIndex> = BTreeSet::new();
        let mut queue: VecDeque<(NodeIndex, usize)> = VecDeque::new();

        // Find all function nodes with this name
        for (&(ref _file, ref name, ref _sl), &idx) in &self.func_index {
            if name == func_name {
                queue.push_back((idx, 0));
                visited.insert(idx);
            }
        }

        while let Some((node, depth)) = queue.pop_front() {
            if depth > 0 {
                if let Some(fid) = self.to_function_id(node) {
                    result.push((fid, depth));
                }
            }

            if depth >= max_depth {
                continue;
            }

            // Follow Return edges (callee → caller) to find callers
            for edge in self.graph.edges(node) {
                if matches!(edge.weight(), CpgEdge::Return) {
                    let caller_idx = edge.target();
                    if !visited.contains(&caller_idx) {
                        visited.insert(caller_idx);
                        queue.push_back((caller_idx, depth + 1));
                    }
                }
            }
        }

        result
    }

    /// Find all callees of a function by name and file, up to a given depth.
    /// Returns (FunctionId, depth) pairs. Equivalent to `CallGraph::callees_of()`.
    pub fn callees_of(
        &self,
        func_name: &str,
        file: &str,
        max_depth: usize,
    ) -> Vec<(FunctionId, usize)> {
        let mut result = Vec::new();
        let mut visited: BTreeSet<NodeIndex> = BTreeSet::new();
        let mut queue: VecDeque<(NodeIndex, usize)> = VecDeque::new();

        // Find the starting function node
        if let Some(idx) = self.function_node(file, func_name) {
            queue.push_back((idx, 0));
            visited.insert(idx);
        }

        while let Some((node, depth)) = queue.pop_front() {
            if depth > 0 {
                if let Some(fid) = self.to_function_id(node) {
                    result.push((fid, depth));
                }
            }

            if depth >= max_depth {
                continue;
            }

            // Follow Call edges to find callees
            for edge in self.graph.edges(node) {
                if matches!(edge.weight(), CpgEdge::Call) {
                    let callee_idx = edge.target();
                    if !visited.contains(&callee_idx) {
                        visited.insert(callee_idx);
                        queue.push_back((callee_idx, depth + 1));
                    }
                }
            }
        }

        result
    }

    /// Find callers that resolve to a function in a specific target file.
    /// Equivalent to `CallGraph::callers_of_in_file()`.
    pub fn callers_of_in_file(
        &self,
        func_name: &str,
        max_depth: usize,
        target_file: Option<&str>,
    ) -> Vec<(FunctionId, usize)> {
        if target_file.is_none() {
            return self.callers_of(func_name, max_depth);
        }
        let tf = target_file.unwrap();

        let mut result = Vec::new();
        let mut visited: BTreeSet<NodeIndex> = BTreeSet::new();
        let mut queue: VecDeque<(NodeIndex, usize)> = VecDeque::new();

        // Start from function nodes with this name in the target file
        for (&(ref file, ref name, ref _sl), &idx) in &self.func_index {
            if name == func_name && file == tf {
                queue.push_back((idx, 0));
                visited.insert(idx);
            }
        }

        while let Some((node, depth)) = queue.pop_front() {
            if depth > 0 {
                if let Some(fid) = self.to_function_id(node) {
                    result.push((fid, depth));
                }
            }

            if depth >= max_depth {
                continue;
            }

            for edge in self.graph.edges(node) {
                if matches!(edge.weight(), CpgEdge::Return) {
                    let caller_idx = edge.target();
                    if !visited.contains(&caller_idx) {
                        visited.insert(caller_idx);
                        queue.push_back((caller_idx, depth + 1));
                    }
                }
            }
        }

        result
    }

    // -----------------------------------------------------------------------
    // DataFlowGraph-equivalent methods
    // -----------------------------------------------------------------------

    /// Get all definition locations of a variable by base name in a file.
    /// Equivalent to `DataFlowGraph::all_defs_of()`.
    pub fn all_defs_of(&self, file: &str, var_name: &str) -> Vec<VarLocation> {
        let mut result = Vec::new();
        for (&(ref f, ref _func, _fsl, ref _line, ref path, ref access), &_idx) in &self.var_index {
            if f == file && path.base == var_name && *access == VarAccess::Def {
                if let Some(loc) = self.to_var_location(_idx) {
                    result.push(loc);
                }
            }
        }
        result
    }

    /// Forward reachability from a VarLocation, following DataFlow edges.
    /// Equivalent to `DataFlowGraph::forward_reachable()`.
    ///
    /// Also handles assignment propagation: if a Use is found, finds all Defs
    /// on the same line (x = y means use of y flows to def of x).
    pub fn dfg_forward_reachable(&self, from: &VarLocation) -> BTreeSet<VarLocation> {
        let from_access = match from.kind {
            VarAccessKind::Def => VarAccess::Def,
            VarAccessKind::Use => VarAccess::Use,
        };
        let start = match self.var_node(
            &from.file,
            &from.function,
            from.function_start_line,
            from.line,
            &from.path,
            from_access,
        ) {
            Some(idx) => idx,
            None => return BTreeSet::new(),
        };

        // BFS following DataFlow edges + same-line assignment propagation
        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(start);

        while let Some(node) = queue.pop_front() {
            if !visited.insert(node) {
                continue;
            }

            // Follow DataFlow edges
            for edge in self.graph.edges(node) {
                if matches!(edge.weight(), CpgEdge::DataFlow) && !visited.contains(&edge.target()) {
                    queue.push_back(edge.target());
                }
            }

            // Assignment propagation: Use on line N → find Defs on same line
            if let CpgNode::Variable {
                access: VarAccess::Use,
                file,
                line,
                ..
            } = &self.graph[node]
            {
                if let Some(nodes_at) = self.location_index.get(&(file.clone(), *line)) {
                    for &other in nodes_at {
                        if let CpgNode::Variable {
                            access: VarAccess::Def,
                            ..
                        } = &self.graph[other]
                        {
                            if !visited.contains(&other) {
                                queue.push_back(other);
                            }
                        }
                    }
                }
            }
        }

        visited.remove(&start);
        visited
            .into_iter()
            .filter_map(|idx| self.to_var_location(idx))
            .collect()
    }

    /// Backward reachability from a VarLocation, following DataFlow edges.
    /// Equivalent to `DataFlowGraph::backward_reachable()`.
    pub fn dfg_backward_reachable(&self, from: &VarLocation) -> BTreeSet<VarLocation> {
        let from_access = match from.kind {
            VarAccessKind::Def => VarAccess::Def,
            VarAccessKind::Use => VarAccess::Use,
        };
        let start = match self.var_node(
            &from.file,
            &from.function,
            from.function_start_line,
            from.line,
            &from.path,
            from_access,
        ) {
            Some(idx) => idx,
            None => return BTreeSet::new(),
        };

        let reachable = self.reachable_backward(start, &|e| matches!(e, CpgEdge::DataFlow));
        reachable
            .into_iter()
            .filter_map(|idx| self.to_var_location(idx))
            .collect()
    }

    /// Forward taint propagation from a set of tainted locations.
    /// Equivalent to `DataFlowGraph::taint_forward()`.
    pub fn taint_forward(
        &self,
        taint_sources: &[(String, usize)],
    ) -> Vec<crate::data_flow::FlowPath> {
        let mut paths = Vec::new();

        for (file, line) in taint_sources {
            let source_nodes = self.nodes_at(file, *line);
            for &src_idx in &source_nodes {
                if !matches!(self.graph[src_idx], CpgNode::Variable { .. }) {
                    continue;
                }
                let src_loc = match self.to_var_location(src_idx) {
                    Some(loc) => loc,
                    None => continue,
                };
                let reachable = self.dfg_forward_reachable(&src_loc);
                if !reachable.is_empty() {
                    let path = crate::data_flow::FlowPath {
                        edges: reachable
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

    /// Find all statements on any data flow path between source and sink.
    /// Equivalent to `DataFlowGraph::chop()`.
    pub fn dfg_chop(
        &self,
        source_file: &str,
        source_line: usize,
        sink_file: &str,
        sink_line: usize,
    ) -> BTreeSet<(String, usize)> {
        let source_nodes: Vec<NodeIndex> = self
            .nodes_at(source_file, source_line)
            .into_iter()
            .filter(|&idx| matches!(self.graph[idx], CpgNode::Variable { .. }))
            .collect();
        let sink_nodes: Vec<NodeIndex> = self
            .nodes_at(sink_file, sink_line)
            .into_iter()
            .filter(|&idx| matches!(self.graph[idx], CpgNode::Variable { .. }))
            .collect();

        let on_path = self.chop(&source_nodes, &sink_nodes, &|e| {
            matches!(e, CpgEdge::DataFlow)
        });

        let mut result: BTreeSet<(String, usize)> = on_path
            .iter()
            .map(|&idx| {
                let node = self.node(idx);
                (node.file().to_string(), node.line())
            })
            .collect();

        result.insert((source_file.to_string(), source_line));
        result.insert((sink_file.to_string(), sink_line));
        result
    }

    // -----------------------------------------------------------------------
    // Type-enriched queries (require TypeDatabase)
    // -----------------------------------------------------------------------

    /// Get all known fields of a record type, including inherited fields.
    ///
    /// Returns None if no TypeDatabase is present or the type is unknown.
    pub fn all_fields_of(&self, type_name: &str) -> Option<Vec<String>> {
        let db = self.type_db.as_ref()?;
        let record = db.resolve_record(type_name)?;
        Some(
            db.all_fields(&record.name)
                .iter()
                .map(|f| f.name.clone())
                .collect(),
        )
    }

    /// Resolve a typedef to its canonical underlying type.
    ///
    /// Returns the input unchanged if no TypeDatabase is present.
    pub fn resolve_type(&self, type_name: &str) -> String {
        match &self.type_db {
            Some(db) => db.resolve_typedef(type_name),
            None => type_name.to_string(),
        }
    }

    /// Check if a type is a union (fields alias each other).
    pub fn is_union_type(&self, type_name: &str) -> bool {
        self.type_db
            .as_ref()
            .is_some_and(|db| db.is_union(type_name))
    }

    /// Get the type of a specific field in a record.
    pub fn field_type(&self, record_name: &str, field_name: &str) -> Option<String> {
        self.type_db.as_ref()?.field_type(record_name, field_name)
    }

    /// Check if type enrichment is available.
    pub fn has_type_info(&self) -> bool {
        self.type_db.is_some()
    }
}
