//! The `CodePropertyGraph` struct definition and its construction methods:
//! building the unified graph from parsed files, assembling nodes/edges, and
//! statement classification.

use crate::access_path::AccessPath;
use crate::call_graph::CallGraph;
use crate::cfg;
use crate::data_flow::{DataFlowGraph, VarAccessKind};
use crate::type_db::TypeDatabase;

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::{BTreeMap, BTreeSet};

use super::{CpgEdge, CpgNode, StmtKind, VarAccess};
use crate::ast::ParsedFile;

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
    pub(crate) func_index: BTreeMap<(String, String), NodeIndex>,

    /// Index: VarLocation-like key → variable node index.
    /// Key: (file, function, line, path, access_kind)
    pub(crate) var_index: BTreeMap<(String, String, usize, AccessPath, VarAccess), NodeIndex>,

    /// Index: (file, line) → all node indices at that location
    pub(crate) location_index: BTreeMap<(String, usize), Vec<NodeIndex>>,

    /// Retained call graph for call site line lookups.
    /// Algorithms like membrane_slice and echo_slice need specific call site
    /// locations (which line does caller X call callee Y on), which the CPG's
    /// Function→Function Call edges don't capture.
    pub call_graph: CallGraph,

    /// Retained data flow graph for direct edge access.
    /// Used by delta_slice for edge diffing.
    pub dfg: DataFlowGraph,

    /// Optional type database for C/C++ type enrichment.
    /// When present, enables precise whole-struct detection, typedef resolution,
    /// field enumeration, and virtual dispatch via class hierarchy analysis.
    pub type_db: Option<TypeDatabase>,
}

impl CodePropertyGraph {
    /// Build a CPG from parsed files, with optional type enrichment.
    ///
    /// When `type_db` is provided, the CPG gains virtual dispatch Call edges
    /// (via CHA) and type-aware query methods. When `None`, behavior is
    /// identical to an unenriched build.
    ///
    /// Constructs the graph by:
    /// 1. Building DataFlowGraph and CallGraph from the same parsed files
    /// 2. Creating Function nodes for each function definition
    /// 3. Creating Variable nodes for each def/use from the DFG
    /// 4. Adding DataFlow edges from DFG edges
    /// 5. Adding Call edges from the call graph
    /// 6. Adding Contains edges (function → its variables)
    /// 7. (If type_db) Adding virtual dispatch Call edges
    /// Build a CPG without type enrichment.
    pub fn build(files: &BTreeMap<String, ParsedFile>) -> Self {
        Self::build_enriched(files, None)
    }

    /// Reconstruct a CPG from pre-built parts (used by cache deserialization).
    ///
    /// The caller is responsible for ensuring that indexes are consistent
    /// with the graph contents.
    pub fn from_parts(
        graph: DiGraph<CpgNode, CpgEdge>,
        func_index: BTreeMap<(String, String), NodeIndex>,
        var_index: BTreeMap<(String, String, usize, AccessPath, VarAccess), NodeIndex>,
        location_index: BTreeMap<(String, usize), Vec<NodeIndex>>,
        call_graph: CallGraph,
        dfg: DataFlowGraph,
    ) -> Self {
        CodePropertyGraph {
            graph,
            func_index,
            var_index,
            location_index,
            call_graph,
            dfg,
            type_db: None,
        }
    }

    /// Create an empty CPG with no nodes or edges.
    ///
    /// Used by `CpgContext::without_cpg` for AST-only algorithms.
    pub fn empty() -> Self {
        CodePropertyGraph {
            graph: DiGraph::new(),
            func_index: BTreeMap::new(),
            var_index: BTreeMap::new(),
            location_index: BTreeMap::new(),
            call_graph: CallGraph::empty(),
            dfg: DataFlowGraph::empty(),
            type_db: None,
        }
    }

    /// Build a CPG with optional type enrichment.
    ///
    /// When `type_db` is `Some`, virtual dispatch Call edges are added via CHA
    /// and type-aware queries become available. When `None`, identical to `build()`.
    pub fn build_enriched(
        files: &BTreeMap<String, ParsedFile>,
        type_db: Option<&TypeDatabase>,
    ) -> Self {
        Self::build_impl(files, type_db.cloned())
    }

    fn build_impl(files: &BTreeMap<String, ParsedFile>, type_db: Option<TypeDatabase>) -> Self {
        let dfg = DataFlowGraph::build(files);
        let cg = CallGraph::build(files);
        Self::assemble_graph(cg, dfg, files, type_db)
    }

    /// Build a CPG with type enrichment from a TypeDatabase.
    ///
    /// Convenience method — equivalent to `build_impl(files, Some(type_db))`.
    pub fn build_with_types(files: &BTreeMap<String, ParsedFile>, type_db: TypeDatabase) -> Self {
        Self::build_impl(files, Some(type_db))
    }

    /// Build a CPG incrementally from a cached CG/DFG and a set of changed files.
    ///
    /// 1. Removes data for changed files from the cached CG/DFG.
    /// 2. Builds fresh CG/DFG for only the changed files.
    /// 3. Merges the fresh data into the retained cached data.
    /// 4. Assembles the full petgraph from the merged CG/DFG.
    ///
    /// The graph assembly (Steps 1–9) runs on the merged CG/DFG, so all
    /// cross-file edges (interprocedural DFG, call edges) are correct.
    /// Build a CPG incrementally by merging cached data with fresh analysis
    /// of changed files.
    ///
    /// # Limitations
    ///
    /// Indirect call resolution (Phase 3: function pointers, struct callbacks)
    /// is NOT re-run for changed files. This is correct for direct-call
    /// languages (Python, JS, Go, Java) but may miss indirect targets in C/C++
    /// code. Use `--no-cache` for C/C++ reviews with heavy function pointer
    /// usage.
    pub fn build_incremental(
        mut cached_cg: CallGraph,
        mut cached_dfg: DataFlowGraph,
        changed_files: &BTreeSet<String>,
        files: &BTreeMap<String, ParsedFile>,
        type_db: Option<TypeDatabase>,
    ) -> Self {
        // Step 1: Remove stale data for changed files.
        cached_cg.remove_files(changed_files);
        cached_dfg.remove_files(changed_files);

        // Step 2: Build fresh CG/DFG for changed files only.
        // Note: build_direct_subset only resolves direct calls (Phases 1+2),
        // not indirect calls (Phase 3: function pointers, parameter-passed
        // callbacks, struct field callbacks). Cached indirect resolution for
        // unchanged files is preserved via the merge.
        let fresh_cg = CallGraph::build_direct_subset(files, changed_files);
        let fresh_dfg = DataFlowGraph::build_subset(files, changed_files);

        // Step 3: Merge fresh into retained.
        cached_cg.merge(fresh_cg);
        cached_dfg.merge(fresh_dfg);

        // Step 4: Assemble the petgraph from the merged CG/DFG.
        // This reuses the same assembly logic as build_impl (Steps 1–9).
        Self::assemble_graph(cached_cg, cached_dfg, files, type_db)
    }

    /// Assemble a CPG petgraph from pre-built CG and DFG.
    ///
    /// Shared by `build_impl` (full build) and `build_incremental` (partial).
    /// Runs Steps 1–9 of graph construction.
    fn assemble_graph(
        cg: CallGraph,
        dfg: DataFlowGraph,
        files: &BTreeMap<String, ParsedFile>,
        type_db: Option<TypeDatabase>,
    ) -> Self {
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

        // --- Steps 2-3: Variable nodes from DFG ---
        for locs in dfg.defs.values() {
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
        for locs in dfg.uses.values() {
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

        // --- Step 4: DataFlow edges ---
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

        // --- Step 5: Call edges ---
        for (caller_id, sites) in &cg.calls {
            let caller_key = (caller_id.file.clone(), caller_id.name.clone());
            let caller_idx = match func_index.get(&caller_key) {
                Some(&idx) => idx,
                None => continue,
            };
            for site in sites {
                // S3: Exact + NameOnly included; drops excluded.
                for resolved in cg.resolve_call_site(site) {
                    let callee_id = resolved.target;
                    let callee_key = (callee_id.file.clone(), callee_id.name.clone());
                    if let Some(&callee_idx) = func_index.get(&callee_key) {
                        graph.add_edge(caller_idx, callee_idx, CpgEdge::Call);
                        graph.add_edge(callee_idx, caller_idx, CpgEdge::Return);
                    }
                }
            }
        }

        // --- Step 5b: Interprocedural data flow edges ---
        for (caller_id, sites) in &cg.calls {
            for site in sites {
                for resolved in cg.resolve_call_site(site) {
                    let callee_id = resolved.target;
                    let caller_parsed = match files.get(&caller_id.file) {
                        Some(p) => p,
                        None => continue,
                    };
                    let arg_texts = caller_parsed.call_argument_texts(site.line, &site.callee_name);
                    if arg_texts.is_empty() {
                        continue;
                    }
                    let callee_parsed = match files.get(&callee_id.file) {
                        Some(p) => p,
                        None => continue,
                    };
                    let Some(info) = callee_parsed
                        .functions()
                        .iter()
                        // first name match wins — pinned-until-S2 (see step5b_param_binding_first_wins_parity)
                        .find(|f| f.name.as_deref() == Some(callee_id.name.as_str()))
                    else {
                        continue;
                    };
                    // S3 (spec §3.3): a Python METHOD's self/cls receiver never binds
                    // to an explicit call arg. Gate on actual ownership — a free
                    // function whose first param merely happens to be named `self`
                    // must keep all its params (else arg→param edges shift by one).
                    let param_names = match info.param_names.first().map(String::as_str) {
                        Some("self") | Some("cls")
                            if info.owner.is_some()
                                && callee_parsed.language == crate::languages::Language::Python =>
                        {
                            &info.param_names[1..]
                        }
                        _ => &info.param_names[..],
                    };
                    for (i, param_name) in param_names.iter().enumerate() {
                        if i >= arg_texts.len() {
                            break;
                        }
                        let arg_text = &arg_texts[i];
                        let arg_base = arg_text.split('.').next().unwrap_or(arg_text);
                        let arg_base = arg_base.split("->").next().unwrap_or(arg_base);
                        let arg_path = AccessPath::simple(arg_base);
                        let arg_key = (
                            caller_id.file.clone(),
                            caller_id.name.clone(),
                            site.line,
                            arg_path.clone(),
                            VarAccess::Use,
                        );
                        let arg_idx = var_index.get(&arg_key).copied().or_else(|| {
                            let def_key = (
                                caller_id.file.clone(),
                                caller_id.name.clone(),
                                site.line,
                                arg_path,
                                VarAccess::Def,
                            );
                            var_index.get(&def_key).copied()
                        });
                        let param_path = AccessPath::simple(param_name);
                        let param_idx =
                            (callee_id.start_line..=callee_id.end_line).find_map(|line| {
                                let key = (
                                    callee_id.file.clone(),
                                    callee_id.name.clone(),
                                    line,
                                    param_path.clone(),
                                    VarAccess::Def,
                                );
                                var_index.get(&key).copied()
                            });
                        if let (Some(from), Some(to)) = (arg_idx, param_idx) {
                            graph.add_edge(from, to, CpgEdge::DataFlow);
                        }
                    }
                }
            }
        }

        // --- Step 6: Contains edges ---
        for (&(ref file, ref func, ref _line, ref _path, ref _access), &var_idx) in &var_index {
            let func_key = (file.clone(), func.clone());
            if let Some(&func_idx) = func_index.get(&func_key) {
                graph.add_edge(func_idx, var_idx, CpgEdge::Contains);
            }
        }

        // --- Step 7: Statement nodes for CFG ---
        let mut stmt_index: BTreeMap<(String, usize), NodeIndex> = BTreeMap::new();
        for (path, parsed) in files {
            let root = parsed.tree.root_node();
            let func_types = parsed.language.function_node_types();
            Self::collect_function_statements(
                root,
                &func_types,
                parsed,
                path,
                &mut graph,
                &mut stmt_index,
                &mut location_index,
            );
        }

        // --- Step 8: ControlFlow edges ---
        for (_path, parsed) in files {
            let cfg_edges = cfg::build_cfg_edges(parsed);
            for edge in cfg_edges {
                let from_idx = stmt_index.get(&(edge.file.clone(), edge.from_line));
                let to_idx = stmt_index.get(&(edge.file.clone(), edge.to_line));
                if let (Some(&from), Some(&to)) = (from_idx, to_idx) {
                    graph.add_edge(from, to, CpgEdge::ControlFlow);
                }
            }
        }

        // --- Step 9: Virtual dispatch enrichment ---
        if let Some(ref tdb) = type_db {
            let mut virtual_edges: Vec<(NodeIndex, NodeIndex)> = Vec::new();
            let live_classes = TypeDatabase::collect_live_classes(files);
            let mut virtual_method_nodes: BTreeMap<String, Vec<(String, NodeIndex)>> =
                BTreeMap::new();
            for record in tdb.records.values() {
                for method_name in record.virtual_methods.keys() {
                    for (&(ref _file, ref name), &idx) in &func_index {
                        if name == method_name {
                            virtual_method_nodes
                                .entry(method_name.clone())
                                .or_default()
                                .push((record.name.clone(), idx));
                        }
                    }
                }
            }
            let all_func_nodes: Vec<NodeIndex> = func_index.values().copied().collect();
            for &caller_idx in &all_func_nodes {
                let callees: Vec<_> = graph
                    .edges(caller_idx)
                    .filter(|e| matches!(e.weight(), CpgEdge::Call))
                    .map(|e| e.target())
                    .collect();
                for callee_idx in callees {
                    if let CpgNode::Function { name, .. } = &graph[callee_idx] {
                        if let Some(override_entries) = virtual_method_nodes.get(name) {
                            for (class_name, override_idx) in override_entries {
                                if *override_idx == callee_idx {
                                    continue;
                                }
                                if !live_classes.is_empty() && !live_classes.contains(class_name) {
                                    continue;
                                }
                                virtual_edges.push((caller_idx, *override_idx));
                            }
                        }
                    }
                }
            }
            for (from, to) in &virtual_edges {
                let already_exists = graph
                    .edges(*from)
                    .any(|e| e.target() == *to && matches!(e.weight(), CpgEdge::Call));
                if !already_exists {
                    graph.add_edge(*from, *to, CpgEdge::Call);
                    graph.add_edge(*to, *from, CpgEdge::Return);
                }
            }
        }

        CodePropertyGraph {
            graph,
            func_index,
            var_index,
            location_index,
            call_graph: cg,
            dfg,
            type_db,
        }
    }

    // -----------------------------------------------------------------------
    // CFG construction helpers (Phase 6)
    // -----------------------------------------------------------------------

    /// Collect all statement-level AST nodes within functions and create
    /// `CpgNode::Statement` nodes in the graph.
    fn collect_function_statements(
        node: tree_sitter::Node<'_>,
        func_types: &[&str],
        parsed: &ParsedFile,
        file: &str,
        graph: &mut DiGraph<CpgNode, CpgEdge>,
        stmt_index: &mut BTreeMap<(String, usize), NodeIndex>,
        location_index: &mut BTreeMap<(String, usize), Vec<NodeIndex>>,
    ) {
        if func_types.contains(&node.kind()) {
            let stmts = parsed.statements_in_function(&node);
            for (line, kind_str) in stmts {
                let key = (file.to_string(), line);
                if stmt_index.contains_key(&key) {
                    continue;
                }
                let kind = Self::classify_stmt_kind(&kind_str, parsed, line);
                let idx = graph.add_node(CpgNode::Statement {
                    file: file.to_string(),
                    line,
                    kind,
                });
                stmt_index.insert(key.clone(), idx);
                location_index.entry(key).or_default().push(idx);
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::collect_function_statements(
                child,
                func_types,
                parsed,
                file,
                graph,
                stmt_index,
                location_index,
            );
        }
    }

    /// Classify a tree-sitter node kind string into a `StmtKind`.
    fn classify_stmt_kind(kind_str: &str, parsed: &ParsedFile, line: usize) -> StmtKind {
        if parsed.language.is_return_node(kind_str) {
            return StmtKind::Return;
        }
        if kind_str == "goto_statement" {
            // Extract target label
            let target = Self::extract_goto_target(parsed, line);
            return StmtKind::Goto {
                target: target.unwrap_or_default(),
            };
        }
        if kind_str == "labeled_statement" {
            let name = Self::extract_label_name(parsed, line);
            return StmtKind::Label {
                name: name.unwrap_or_default(),
            };
        }
        if parsed.language.is_loop_node(kind_str) {
            return StmtKind::Loop;
        }
        if parsed.language.is_control_flow_node(kind_str) {
            return StmtKind::Branch;
        }
        if parsed.language.is_assignment_node(kind_str) {
            return StmtKind::Assignment;
        }
        if parsed.language.is_declaration_node(kind_str) {
            return StmtKind::Declaration;
        }
        if parsed.language.is_call_node(kind_str) || kind_str == "expression_statement" {
            // expression_statement often wraps a call
            let calls = parsed.call_names_on_lines(&[line]);
            if let Some(names) = calls.get(&line) {
                if let Some(callee) = names.first() {
                    return StmtKind::Call {
                        callee: callee.clone(),
                    };
                }
            }
        }
        StmtKind::Other
    }

    fn extract_goto_target(parsed: &ParsedFile, line: usize) -> Option<String> {
        let root = parsed.tree.root_node();
        Self::find_goto_at_line(root, parsed, line)
    }

    fn find_goto_at_line(
        node: tree_sitter::Node<'_>,
        parsed: &ParsedFile,
        line: usize,
    ) -> Option<String> {
        if node.kind() == "goto_statement" && node.start_position().row + 1 == line {
            if let Some(label) = node.child_by_field_name("label") {
                return Some(parsed.node_text(&label).to_string());
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = Self::find_goto_at_line(child, parsed, line) {
                return Some(found);
            }
        }
        None
    }

    fn extract_label_name(parsed: &ParsedFile, line: usize) -> Option<String> {
        let root = parsed.tree.root_node();
        Self::find_label_at_line(root, parsed, line)
    }

    fn find_label_at_line(
        node: tree_sitter::Node<'_>,
        parsed: &ParsedFile,
        line: usize,
    ) -> Option<String> {
        if node.kind() == "labeled_statement" && node.start_position().row + 1 == line {
            if let Some(label) = node.child_by_field_name("label") {
                return Some(parsed.node_text(&label).to_string());
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = Self::find_label_at_line(child, parsed, line) {
                return Some(found);
            }
        }
        None
    }
}
