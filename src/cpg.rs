//! Code Property Graph — unified graph merging AST, DFG, call graph, and (future) CFG.
//!
//! Built on `petgraph`, this module provides:
//! - **Schema types:** `CpgNode`, `CpgEdge` — node and edge types for the unified graph
//! - **Builder:** `CodePropertyGraph::build()` — constructs the graph from parsed files
//! - **Query methods:** edge-filtered reachability, SCC, shortest paths, subgraph views
//!
//! Algorithms can query the CPG instead of separately accessing `DataFlowGraph`,
//! `CallGraph`, and `ast.rs`. Edge-filtered traversals let each algorithm select
//! which relationship types to follow.
//!
//! See `docs/cpg-architecture.md` for the full design.

use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::call_graph::{CallGraph, FunctionId};
use crate::data_flow::{DataFlowGraph, VarAccessKind, VarLocation};

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

// ---------------------------------------------------------------------------
// Node types
// ---------------------------------------------------------------------------

/// A node in the Code Property Graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CpgNode {
    /// A function definition.
    Function {
        name: String,
        file: String,
        start_line: usize,
        end_line: usize,
    },

    /// A statement or expression at a specific source location.
    Statement {
        file: String,
        line: usize,
        kind: StmtKind,
    },

    /// A variable access (definition or use) with a structured access path.
    Variable {
        path: AccessPath,
        file: String,
        function: String,
        line: usize,
        access: VarAccess,
    },
}

/// Classification of statements relevant for analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StmtKind {
    /// Variable assignment: `x = expr`
    Assignment,
    /// Function/method call.
    Call { callee: String },
    /// Return statement.
    Return,
    /// Conditional branch: if, switch, match.
    Branch,
    /// Loop: for, while, loop, do-while.
    Loop,
    /// Goto statement (C/C++).
    Goto { target: String },
    /// Label (C/C++ goto target).
    Label { name: String },
    /// Variable/type declaration.
    Declaration,
    /// Any other statement.
    Other,
}

/// Whether a variable access is a definition (write) or use (read).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VarAccess {
    /// Variable is written to (assigned, declared with initializer).
    Def,
    /// Variable is read.
    Use,
}

// ---------------------------------------------------------------------------
// Edge types
// ---------------------------------------------------------------------------

/// An edge in the Code Property Graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CpgEdge {
    /// Data flow: a definition reaches this use (def-use chain).
    DataFlow,

    /// Control flow: execution can proceed from source to target.
    /// Added in Phase 6.
    ControlFlow,

    /// Call: a call site invokes a callee function.
    Call,

    /// Return: a function returns to the call site.
    Return,

    /// Containment: a function contains this statement or variable.
    Contains,

    /// Field relationship: a variable is a field access on another variable.
    FieldOf,
}

// ---------------------------------------------------------------------------
// Node accessors
// ---------------------------------------------------------------------------

impl CpgNode {
    /// The file path this node belongs to.
    pub fn file(&self) -> &str {
        match self {
            CpgNode::Function { file, .. } => file,
            CpgNode::Statement { file, .. } => file,
            CpgNode::Variable { file, .. } => file,
        }
    }

    /// The primary line number of this node.
    pub fn line(&self) -> usize {
        match self {
            CpgNode::Function { start_line, .. } => *start_line,
            CpgNode::Statement { line, .. } => *line,
            CpgNode::Variable { line, .. } => *line,
        }
    }

    /// Whether this node is a function definition.
    pub fn is_function(&self) -> bool {
        matches!(self, CpgNode::Function { .. })
    }

    /// Whether this node is a variable definition.
    pub fn is_def(&self) -> bool {
        matches!(
            self,
            CpgNode::Variable {
                access: VarAccess::Def,
                ..
            }
        )
    }

    /// Whether this node is a variable use.
    pub fn is_use(&self) -> bool {
        matches!(
            self,
            CpgNode::Variable {
                access: VarAccess::Use,
                ..
            }
        )
    }

    /// Whether this node is a call statement.
    pub fn is_call(&self) -> bool {
        matches!(
            self,
            CpgNode::Statement {
                kind: StmtKind::Call { .. },
                ..
            }
        )
    }
}

impl CpgEdge {
    /// Whether this is a data flow edge.
    pub fn is_data_flow(&self) -> bool {
        matches!(self, CpgEdge::DataFlow)
    }

    /// Whether this is a call or return edge.
    pub fn is_interprocedural(&self) -> bool {
        matches!(self, CpgEdge::Call | CpgEdge::Return)
    }
}

// ---------------------------------------------------------------------------
// Code Property Graph
// ---------------------------------------------------------------------------

/// The unified Code Property Graph.
///
/// Merges data flow, call graph, and containment relationships into a single
/// petgraph DiGraph. Algorithms query this graph with edge-type filters instead
/// of separately accessing DataFlowGraph and CallGraph.
pub struct CodePropertyGraph {
    /// The underlying petgraph directed graph.
    pub graph: DiGraph<CpgNode, CpgEdge>,

    /// Index: (file, function_name) → function node index
    func_index: BTreeMap<(String, String), NodeIndex>,

    /// Index: VarLocation-like key → variable node index.
    /// Key: (file, function, line, path, access_kind)
    var_index: BTreeMap<(String, String, usize, AccessPath, VarAccess), NodeIndex>,

    /// Index: (file, line) → all node indices at that location
    location_index: BTreeMap<(String, usize), Vec<NodeIndex>>,

    /// Retained call graph for call site line lookups.
    /// Algorithms like membrane_slice and echo_slice need specific call site
    /// locations (which line does caller X call callee Y on), which the CPG's
    /// Function→Function Call edges don't capture.
    pub call_graph: CallGraph,

    /// Retained data flow graph for direct edge access.
    /// Used by delta_slice for edge diffing.
    pub dfg: DataFlowGraph,
}

impl CodePropertyGraph {
    /// Build a CPG from parsed files.
    ///
    /// Constructs the graph by:
    /// 1. Building DataFlowGraph and CallGraph from the same parsed files
    /// 2. Creating Function nodes for each function definition
    /// 3. Creating Variable nodes for each def/use from the DFG
    /// 4. Adding DataFlow edges from DFG edges
    /// 5. Adding Call edges from the call graph
    /// 6. Adding Contains edges (function → its variables)
    pub fn build(files: &BTreeMap<String, ParsedFile>) -> Self {
        let dfg = DataFlowGraph::build(files);
        let cg = CallGraph::build(files);

        let mut graph = DiGraph::new();
        let mut func_index: BTreeMap<(String, String), NodeIndex> = BTreeMap::new();
        let mut var_index: BTreeMap<(String, String, usize, AccessPath, VarAccess), NodeIndex> =
            BTreeMap::new();
        let mut location_index: BTreeMap<(String, usize), Vec<NodeIndex>> = BTreeMap::new();

        // --- Step 1: Function nodes ---
        for func_ids in cg.functions.values() {
            for fid in func_ids {
                let idx = graph.add_node(CpgNode::Function {
                    name: fid.name.clone(),
                    file: fid.file.clone(),
                    start_line: fid.start_line,
                    end_line: fid.end_line,
                });
                func_index.insert((fid.file.clone(), fid.name.clone()), idx);
                location_index
                    .entry((fid.file.clone(), fid.start_line))
                    .or_default()
                    .push(idx);
            }
        }

        // --- Step 2: Variable nodes from DFG defs ---
        for ((_file, _func, _path), locs) in &dfg.defs {
            for loc in locs {
                let access = VarAccess::Def;
                let key = (
                    loc.file.clone(),
                    loc.function.clone(),
                    loc.line,
                    loc.path.clone(),
                    access,
                );
                if !var_index.contains_key(&key) {
                    let idx = graph.add_node(CpgNode::Variable {
                        path: loc.path.clone(),
                        file: loc.file.clone(),
                        function: loc.function.clone(),
                        line: loc.line,
                        access,
                    });
                    var_index.insert(key, idx);
                    location_index
                        .entry((loc.file.clone(), loc.line))
                        .or_default()
                        .push(idx);
                }
            }
        }

        // --- Step 3: Variable nodes from DFG uses ---
        for ((_file, _func, _path), locs) in &dfg.uses {
            for loc in locs {
                let access = VarAccess::Use;
                let key = (
                    loc.file.clone(),
                    loc.function.clone(),
                    loc.line,
                    loc.path.clone(),
                    access,
                );
                if !var_index.contains_key(&key) {
                    let idx = graph.add_node(CpgNode::Variable {
                        path: loc.path.clone(),
                        file: loc.file.clone(),
                        function: loc.function.clone(),
                        line: loc.line,
                        access,
                    });
                    var_index.insert(key, idx);
                    location_index
                        .entry((loc.file.clone(), loc.line))
                        .or_default()
                        .push(idx);
                }
            }
        }

        // --- Step 4: DataFlow edges from DFG ---
        for edge in &dfg.edges {
            let from_access = match edge.from.kind {
                VarAccessKind::Def => VarAccess::Def,
                VarAccessKind::Use => VarAccess::Use,
            };
            let to_access = match edge.to.kind {
                VarAccessKind::Def => VarAccess::Def,
                VarAccessKind::Use => VarAccess::Use,
            };
            let from_key = (
                edge.from.file.clone(),
                edge.from.function.clone(),
                edge.from.line,
                edge.from.path.clone(),
                from_access,
            );
            let to_key = (
                edge.to.file.clone(),
                edge.to.function.clone(),
                edge.to.line,
                edge.to.path.clone(),
                to_access,
            );
            if let (Some(&from_idx), Some(&to_idx)) =
                (var_index.get(&from_key), var_index.get(&to_key))
            {
                graph.add_edge(from_idx, to_idx, CpgEdge::DataFlow);
            }
        }

        // --- Step 5: Call edges from call graph ---
        for (caller_id, sites) in &cg.calls {
            let caller_key = (caller_id.file.clone(), caller_id.name.clone());
            let caller_idx = match func_index.get(&caller_key) {
                Some(&idx) => idx,
                None => continue,
            };

            for site in sites {
                // Resolve callee to function nodes
                let callee_ids = cg.resolve_callees(&site.callee_name, &caller_id.file);
                for callee_id in callee_ids {
                    let callee_key = (callee_id.file.clone(), callee_id.name.clone());
                    if let Some(&callee_idx) = func_index.get(&callee_key) {
                        graph.add_edge(caller_idx, callee_idx, CpgEdge::Call);
                        graph.add_edge(callee_idx, caller_idx, CpgEdge::Return);
                    }
                }
            }
        }

        // --- Step 6: Contains edges (function → its variables) ---
        for (&(ref file, ref func, ref _line, ref _path, ref _access), &var_idx) in &var_index {
            let func_key = (file.clone(), func.clone());
            if let Some(&func_idx) = func_index.get(&func_key) {
                graph.add_edge(func_idx, var_idx, CpgEdge::Contains);
            }
        }

        // TODO(Phase 4+): Add FieldOf edges connecting field-qualified Variable nodes
        // (e.g., dev.name) to their base Variable nodes (dev). This would enable queries
        // like "which field accesses exist for this struct?" Currently FieldOf is defined
        // in the schema but not constructed.

        CodePropertyGraph {
            graph,
            func_index,
            var_index,
            location_index,
            call_graph: cg,
            dfg,
        }
    }

    // -----------------------------------------------------------------------
    // Index lookups
    // -----------------------------------------------------------------------

    /// Get the node index for a function by file and name.
    pub fn function_node(&self, file: &str, name: &str) -> Option<NodeIndex> {
        self.func_index
            .get(&(file.to_string(), name.to_string()))
            .copied()
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

    /// Get the node index for a variable by its location.
    pub fn var_node(
        &self,
        file: &str,
        function: &str,
        line: usize,
        path: &AccessPath,
        access: VarAccess,
    ) -> Option<NodeIndex> {
        self.var_index
            .get(&(
                file.to_string(),
                function.to_string(),
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
                line,
                access,
            } => Some(VarLocation {
                file: file.clone(),
                function: function.clone(),
                line: *line,
                path: path.clone(),
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
        for (&(ref f, ref _name), &idx) in &self.func_index {
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
        for (&(ref _file, ref name), &idx) in &self.func_index {
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
        if let Some(&idx) = self
            .func_index
            .get(&(file.to_string(), func_name.to_string()))
        {
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
        for (&(ref file, ref name), &idx) in &self.func_index {
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
        for (&(ref f, ref _func, ref _line, ref path, ref access), &_idx) in &self.var_index {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::ParsedFile;
    use crate::languages::Language;

    #[test]
    fn test_node_accessors() {
        let func = CpgNode::Function {
            name: "main".into(),
            file: "src/main.c".into(),
            start_line: 1,
            end_line: 10,
        };
        assert_eq!(func.file(), "src/main.c");
        assert_eq!(func.line(), 1);
        assert!(func.is_function());

        let var_def = CpgNode::Variable {
            path: AccessPath::from_expr("dev->name"),
            file: "src/dev.c".into(),
            function: "init".into(),
            line: 5,
            access: VarAccess::Def,
        };
        assert!(var_def.is_def());
        assert!(!var_def.is_use());

        let call = CpgNode::Statement {
            file: "src/main.c".into(),
            line: 3,
            kind: StmtKind::Call {
                callee: "init".into(),
            },
        };
        assert!(call.is_call());
    }

    #[test]
    fn test_edge_classification() {
        assert!(CpgEdge::DataFlow.is_data_flow());
        assert!(!CpgEdge::Call.is_data_flow());
        assert!(CpgEdge::Call.is_interprocedural());
        assert!(CpgEdge::Return.is_interprocedural());
        assert!(!CpgEdge::DataFlow.is_interprocedural());
        assert!(!CpgEdge::Contains.is_interprocedural());
        assert!(!CpgEdge::FieldOf.is_interprocedural());
        assert!(!CpgEdge::ControlFlow.is_data_flow());
    }

    #[test]
    fn test_variable_node_accessors() {
        let var_use = CpgNode::Variable {
            path: AccessPath::from_expr("dev->id"),
            file: "src/dev.c".into(),
            function: "get_id".into(),
            line: 8,
            access: VarAccess::Use,
        };
        assert!(var_use.is_use());
        assert!(!var_use.is_def());
        assert!(!var_use.is_function());
        assert!(!var_use.is_call());
        assert_eq!(var_use.file(), "src/dev.c");
        assert_eq!(var_use.line(), 8);
    }

    #[test]
    fn test_statement_node_non_call() {
        let branch = CpgNode::Statement {
            file: "src/main.c".into(),
            line: 15,
            kind: StmtKind::Branch,
        };
        assert!(!branch.is_call());
        assert!(!branch.is_function());
        assert!(!branch.is_def());
        assert_eq!(branch.file(), "src/main.c");
        assert_eq!(branch.line(), 15);

        let ret = CpgNode::Statement {
            file: "src/main.c".into(),
            line: 20,
            kind: StmtKind::Return,
        };
        assert!(!ret.is_call());
    }

    #[test]
    fn test_cpg_build_basic() {
        let source = r#"
void init() {
    int x = 1;
    int y = x;
    use(y);
}
"#;
        let path = "src/test.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);

        // Should have at least one function node
        assert!(cpg.node_count() > 0, "CPG should have nodes");
        assert!(cpg.edge_count() > 0, "CPG should have edges");

        // Should be able to look up the function
        let func_idx = cpg.function_node(path, "init");
        assert!(func_idx.is_some(), "Should find function 'init'");

        // Function node should have correct metadata
        let func = cpg.node(func_idx.unwrap());
        assert!(func.is_function());
        assert_eq!(func.file(), path);
    }

    #[test]
    fn test_cpg_dataflow_edges() {
        let source = r#"
void flow() {
    int x = 1;
    int y = x;
}
"#;
        let path = "src/flow.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);

        // Check that dataflow edges exist
        let df_edges: Vec<_> = cpg
            .graph
            .edge_indices()
            .filter(|&e| cpg.graph[e] == CpgEdge::DataFlow)
            .collect();
        assert!(
            !df_edges.is_empty(),
            "CPG should have DataFlow edges for x → y"
        );
    }

    #[test]
    fn test_cpg_call_edges() {
        let source = r#"
void callee() {
    return;
}

void caller() {
    callee();
}
"#;
        let path = "src/calls.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);

        // Check caller → callee Call edge
        let caller_idx = cpg.function_node(path, "caller").unwrap();
        let callee_idx = cpg.function_node(path, "callee").unwrap();

        let call_reachable = cpg.reachable_forward(caller_idx, &|e| matches!(e, CpgEdge::Call));
        assert!(
            call_reachable.contains(&callee_idx),
            "caller should reach callee via Call edge"
        );

        // Check callee → caller Return edge
        let return_reachable = cpg.reachable_forward(callee_idx, &|e| matches!(e, CpgEdge::Return));
        assert!(
            return_reachable.contains(&caller_idx),
            "callee should reach caller via Return edge"
        );
    }

    #[test]
    fn test_cpg_contains_edges() {
        let source = r#"
void f() {
    int x = 1;
    int y = x;
}
"#;
        let path = "src/contains.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);

        let func_idx = cpg.function_node(path, "f").unwrap();
        let contained = cpg.reachable_forward(func_idx, &|e| matches!(e, CpgEdge::Contains));
        assert!(
            !contained.is_empty(),
            "Function 'f' should contain variable nodes"
        );

        // All contained nodes should be Variable nodes
        for idx in &contained {
            let node = cpg.node(*idx);
            assert!(
                node.is_def() || node.is_use(),
                "Contains edge should lead to Variable nodes, got {:?}",
                node
            );
        }
    }

    #[test]
    fn test_cpg_edge_filtered_reachability() {
        // DataFlow-only reachability should NOT follow Call edges
        let source = r#"
void helper() {
    return;
}

void main_func() {
    int x = 1;
    int y = x;
    helper();
}
"#;
        let path = "src/filter.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);

        let main_idx = cpg.function_node(path, "main_func").unwrap();
        let helper_idx = cpg.function_node(path, "helper").unwrap();

        // Call-only should reach helper
        let call_reach = cpg.reachable_forward(main_idx, &|e| matches!(e, CpgEdge::Call));
        assert!(call_reach.contains(&helper_idx));

        // DataFlow-only from main_func should NOT reach helper function node
        let df_reach = cpg.reachable_forward(main_idx, &|e| matches!(e, CpgEdge::DataFlow));
        assert!(
            !df_reach.contains(&helper_idx),
            "DataFlow-only traversal should not reach function nodes via Call edges"
        );
    }

    #[test]
    fn test_cpg_call_graph_cycles() {
        let source = r#"
void a() {
    b();
}

void b() {
    a();
}
"#;
        let path = "src/cycle.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);
        let cycles = cpg.call_graph_cycles();

        assert!(!cycles.is_empty(), "Should detect a → b → a call cycle");

        // The cycle should contain both function nodes
        let cycle_names: BTreeSet<String> = cycles[0]
            .iter()
            .filter_map(|&idx| match cpg.node(idx) {
                CpgNode::Function { name, .. } => Some(name.clone()),
                _ => None,
            })
            .collect();
        assert!(cycle_names.contains("a"), "Cycle should contain 'a'");
        assert!(cycle_names.contains("b"), "Cycle should contain 'b'");
    }

    #[test]
    fn test_cpg_bfs_with_distance() {
        let source = r#"
void a() {
    b();
}

void b() {
    c();
}

void c() {
    return;
}
"#;
        let path = "src/dist.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);

        let a_idx = cpg.function_node(path, "a").unwrap();
        let b_idx = cpg.function_node(path, "b").unwrap();
        let c_idx = cpg.function_node(path, "c").unwrap();

        let distances = cpg.bfs_with_distance(&[a_idx], 5, &|e| matches!(e, CpgEdge::Call));

        assert_eq!(distances.get(&a_idx), Some(&0));
        assert_eq!(distances.get(&b_idx), Some(&1));
        assert_eq!(distances.get(&c_idx), Some(&2));
    }

    #[test]
    fn test_cpg_bridge_to_var_location() {
        let source = r#"
void f() {
    int x = 1;
    int y = x;
}
"#;
        let path = "src/bridge.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);

        // Find a variable node and convert back
        let var_nodes: Vec<_> = cpg
            .graph
            .node_indices()
            .filter(|&idx| cpg.node(idx).is_def())
            .collect();
        assert!(!var_nodes.is_empty());

        let loc = cpg.to_var_location(var_nodes[0]);
        assert!(loc.is_some());
        let loc = loc.unwrap();
        assert_eq!(loc.file, path);
        assert_eq!(loc.function, "f");
    }

    #[test]
    fn test_cpg_bridge_to_function_id() {
        let source = r#"
void my_func() {
    return;
}
"#;
        let path = "src/bridge_func.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);

        let func_idx = cpg.function_node(path, "my_func").unwrap();
        let fid = cpg.to_function_id(func_idx).unwrap();
        assert_eq!(fid.name, "my_func");
        assert_eq!(fid.file, path);
    }
}
