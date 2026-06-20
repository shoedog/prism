//! The `CodePropertyGraph` struct definition and its construction methods:
//! building the unified graph from parsed files, assembling nodes/edges, and
//! statement classification.

use crate::access_path::AccessPath;
use crate::call_graph::{CallGraph, FunctionId, ScopeGraphBuildInputs};
use crate::cfg;
use crate::data_flow::{DataFlowGraph, VarAccessKind};
use crate::resolution::ResolutionConfidence;
use crate::type_db::TypeDatabase;

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::{BTreeMap, BTreeSet};

use super::{CpgEdge, CpgNode, StmtKind, VarAccess};
use crate::ast::ParsedFile;

/// The normalized parameter-name list for a resolved callee, as Step 5b computes it.
/// Pure function of `(callee.file, callee.name, callee.start_line)` + the immutable
/// `callee_parsed` - memoizable per callee. `None` mirrors the current
/// `let Some(info) = ... else { continue }` (callee FunctionInfo not found -> skip the site).
pub(crate) fn compute_param_names(
    callee_parsed: &ParsedFile,
    callee_id: &FunctionId,
) -> Option<Vec<String>> {
    let info = callee_parsed.functions().iter().find(|f| {
        f.name.as_deref() == Some(callee_id.name.as_str()) && f.start_line == callee_id.start_line
    })?;
    // S3 (spec §3.3): a Python METHOD's self/cls receiver never binds to an explicit
    // call arg. Gate on actual ownership — a free function whose first param merely
    // happens to be named `self` must keep all its params.
    let normalized: Vec<String> = callee_parsed
        .all_functions()
        .into_iter()
        .find(|node| {
            callee_parsed
                .language
                .function_name(node)
                .map(|name| callee_parsed.node_text(&name) == callee_id.name.as_str())
                .unwrap_or(false)
                && callee_parsed.node_line_range(node).0 == callee_id.start_line
        })
        .map(|node| {
            callee_parsed
                .function_parameter_occurrences(&node)
                .into_iter()
                .map(|(name, _, _)| name)
                .collect()
        })
        .unwrap_or_else(|| info.param_names.clone());
    let final_names = match normalized.first().map(String::as_str) {
        Some("self") | Some("cls")
            if info.owner.is_some()
                && callee_parsed.language == crate::languages::Language::Python =>
        {
            normalized[1..].to_vec()
        }
        _ => normalized,
    };
    Some(final_names)
}

struct PendingStatement {
    line: usize,
    kind: StmtKind,
    start_byte: usize,
    end_byte: usize,
}

/// An edge pending insertion: (from, to, weight). Collected in deterministic
/// unit order, then applied by a serial `add_edge` loop (S1 C2 pattern).
pub(crate) type PendingEdge = (NodeIndex, NodeIndex, CpgEdge);

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

    /// Index: (file, function_name, start_line) → function node index
    pub(crate) func_index: BTreeMap<(String, String, usize), NodeIndex>,

    /// Index: (file, function_name) → function node indexes sorted by start_line.
    pub(crate) name_index: BTreeMap<(String, String), Vec<NodeIndex>>,

    /// Index: VarLocation-like key → variable node index.
    /// Key: (file, function, function_start_line, line, path, access_kind)
    pub(crate) var_index:
        BTreeMap<(String, String, usize, usize, AccessPath, VarAccess), NodeIndex>,

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
        func_index: BTreeMap<(String, String, usize), NodeIndex>,
        name_index: BTreeMap<(String, String), Vec<NodeIndex>>,
        var_index: BTreeMap<(String, String, usize, usize, AccessPath, VarAccess), NodeIndex>,
        location_index: BTreeMap<(String, usize), Vec<NodeIndex>>,
        call_graph: CallGraph,
        dfg: DataFlowGraph,
    ) -> Self {
        CodePropertyGraph {
            graph,
            func_index,
            name_index,
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
            name_index: BTreeMap::new(),
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
        let inputs = ScopeGraphBuildInputs::from_files_convention(files);
        Self::build_impl(files, type_db.cloned(), Some(&inputs))
    }

    pub fn build_enriched_with_scope_graph_inputs(
        files: &BTreeMap<String, ParsedFile>,
        type_db: Option<&TypeDatabase>,
        scope_inputs: Option<&ScopeGraphBuildInputs>,
    ) -> Self {
        Self::build_impl(files, type_db.cloned(), scope_inputs)
    }

    pub(crate) fn build_enriched_without_scope_graph(
        files: &BTreeMap<String, ParsedFile>,
        type_db: Option<&TypeDatabase>,
    ) -> Self {
        Self::build_impl(files, type_db.cloned(), None)
    }

    fn build_impl(
        files: &BTreeMap<String, ParsedFile>,
        type_db: Option<TypeDatabase>,
        scope_inputs: Option<&ScopeGraphBuildInputs>,
    ) -> Self {
        let dfg = DataFlowGraph::build(files);
        let cg = CallGraph::build_with_scope_graph_inputs(files, scope_inputs);
        Self::assemble_graph(cg, dfg, files, type_db)
    }

    /// Build a CPG with type enrichment from a TypeDatabase.
    ///
    /// Convenience method — equivalent to `build_impl(files, Some(type_db))`.
    pub fn build_with_types(files: &BTreeMap<String, ParsedFile>, type_db: TypeDatabase) -> Self {
        let inputs = ScopeGraphBuildInputs::from_files_convention(files);
        Self::build_impl(files, Some(type_db), Some(&inputs))
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
        cached_cg: CallGraph,
        cached_dfg: DataFlowGraph,
        changed_files: &BTreeSet<String>,
        files: &BTreeMap<String, ParsedFile>,
        type_db: Option<TypeDatabase>,
    ) -> Self {
        let inputs = ScopeGraphBuildInputs::from_files_convention(files);
        Self::build_incremental_with_scope_graph_inputs(
            cached_cg,
            cached_dfg,
            changed_files,
            files,
            type_db,
            Some(&inputs),
        )
    }

    pub fn build_incremental_with_scope_graph_inputs(
        mut cached_cg: CallGraph,
        mut cached_dfg: DataFlowGraph,
        changed_files: &BTreeSet<String>,
        files: &BTreeMap<String, ParsedFile>,
        type_db: Option<TypeDatabase>,
        scope_inputs: Option<&ScopeGraphBuildInputs>,
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

        // Phase-IP: Go embedding promotion is whole-program — recompute (replace-
        // not-merge) over ALL merged files so a removed/changed embedding cannot
        // leave a stale alias (remove_files prunes methods by fid.file only).
        cached_cg.apply_go_embedding_promotion(files);
        cached_cg.apply_go_interface_dispatch(files);
        // Rebuild-together: this also refreshes Phase-2a Rust receiver indices
        // and re-materializes CallSite.receiver_outcome before assemble reads it.
        cached_cg.rebuild_scope_graph(files, scope_inputs);

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
        let mut func_index: BTreeMap<(String, String, usize), NodeIndex> = BTreeMap::new();
        let mut name_index: BTreeMap<(String, String), Vec<NodeIndex>> = BTreeMap::new();
        let mut var_index: BTreeMap<
            (String, String, usize, usize, AccessPath, VarAccess),
            NodeIndex,
        > = BTreeMap::new();
        let mut location_index: BTreeMap<(String, usize), Vec<NodeIndex>> = BTreeMap::new();

        // --- Step 1: Function nodes ---
        for func_ids in cg.functions.values() {
            for fid in func_ids {
                let (start_byte, end_byte) = files
                    .get(&fid.file)
                    .and_then(|p| {
                        p.functions()
                            .iter()
                            .find(|f| {
                                f.name.as_deref() == Some(fid.name.as_str())
                                    && f.start_line == fid.start_line
                            })
                            .map(|f| (f.start_byte, f.end_byte))
                    })
                    .unwrap_or((0, 0));
                let idx = graph.add_node(CpgNode::Function {
                    name: fid.name.clone(),
                    file: fid.file.clone(),
                    start_line: fid.start_line,
                    end_line: fid.end_line,
                    start_byte,
                    end_byte,
                });
                func_index.insert((fid.file.clone(), fid.name.clone(), fid.start_line), idx);
                name_index
                    .entry((fid.file.clone(), fid.name.clone()))
                    .or_default()
                    .push(idx);
                location_index
                    .entry((fid.file.clone(), fid.start_line))
                    .or_default()
                    .push(idx);
            }
        }
        for nodes in name_index.values_mut() {
            nodes.sort_by_key(|&n| match &graph[n] {
                CpgNode::Function { start_line, .. } => *start_line,
                _ => usize::MAX,
            });
        }

        // --- Steps 2-3: Variable nodes from DFG ---
        for locs in dfg.defs.values() {
            for loc in locs {
                let access = VarAccess::Def;
                let key = (
                    loc.file.clone(),
                    loc.function.clone(),
                    loc.function_start_line,
                    loc.line,
                    loc.path.clone(),
                    access,
                );
                if !var_index.contains_key(&key) {
                    let idx = graph.add_node(CpgNode::Variable {
                        path: loc.path.clone(),
                        file: loc.file.clone(),
                        function: loc.function.clone(),
                        function_start_line: loc.function_start_line,
                        line: loc.line,
                        access,
                        start_byte: loc.start_byte,
                        end_byte: loc.end_byte,
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
                    loc.function_start_line,
                    loc.line,
                    loc.path.clone(),
                    access,
                );
                if !var_index.contains_key(&key) {
                    let idx = graph.add_node(CpgNode::Variable {
                        path: loc.path.clone(),
                        file: loc.file.clone(),
                        function: loc.function.clone(),
                        function_start_line: loc.function_start_line,
                        line: loc.line,
                        access,
                        start_byte: loc.start_byte,
                        end_byte: loc.end_byte,
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
                edge.from.function_start_line,
                edge.from.line,
                edge.from.path.clone(),
                from_access,
            );
            let to_key = (
                edge.to.file.clone(),
                edge.to.function.clone(),
                edge.to.function_start_line,
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
            let caller_key = (
                caller_id.file.clone(),
                caller_id.name.clone(),
                caller_id.start_line,
            );
            let caller_idx = match func_index.get(&caller_key) {
                Some(&idx) => idx,
                None => continue,
            };
            for site in sites {
                // S3: Exact + NameOnly included; drops excluded.
                for resolved in cg.resolve_call_site(site) {
                    let callee_id = resolved.target;
                    let callee_key = (
                        callee_id.file.clone(),
                        callee_id.name.clone(),
                        callee_id.start_line,
                    );
                    if let Some(&callee_idx) = func_index.get(&callee_key) {
                        graph.add_edge(caller_idx, callee_idx, CpgEdge::Call(resolved.confidence));
                        graph.add_edge(
                            callee_idx,
                            caller_idx,
                            CpgEdge::Return(resolved.confidence),
                        );
                    }
                }
            }
        }

        // --- Step 5b: Interprocedural data flow edges ---
        let mut param_cache: std::collections::BTreeMap<
            (String, String, usize),
            Option<Vec<String>>,
        > = std::collections::BTreeMap::new();
        for (caller_id, sites) in &cg.calls {
            for site in sites {
                for resolved in cg.resolve_call_site(site) {
                    let callee_id = resolved.target;
                    let caller_parsed = match files.get(&caller_id.file) {
                        Some(p) => p,
                        None => continue,
                    };
                    let arg_texts =
                        caller_parsed.call_argument_texts_at(site.start_byte, &site.callee_name);
                    if arg_texts.is_empty() {
                        continue;
                    }
                    let callee_parsed = match files.get(&callee_id.file) {
                        Some(p) => p,
                        None => continue,
                    };
                    let cache_key = (
                        callee_id.file.clone(),
                        callee_id.name.clone(),
                        callee_id.start_line,
                    );
                    let param_names: &[String] = match param_cache
                        .entry(cache_key)
                        .or_insert_with(|| compute_param_names(callee_parsed, &callee_id))
                    {
                        Some(names) => names.as_slice(),
                        None => continue,
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
                            caller_id.start_line,
                            site.line,
                            arg_path.clone(),
                            VarAccess::Use,
                        );
                        let arg_idx = var_index.get(&arg_key).copied().or_else(|| {
                            let def_key = (
                                caller_id.file.clone(),
                                caller_id.name.clone(),
                                caller_id.start_line,
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
                                    callee_id.start_line,
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
        for (&(ref file, ref func, func_start_line, ref _line, ref _path, ref _access), &var_idx) in
            &var_index
        {
            if let Some(&func_idx) = func_index.get(&(file.clone(), func.clone(), func_start_line))
            {
                graph.add_edge(func_idx, var_idx, CpgEdge::Contains);
            }
        }

        // --- Step 7: Statement nodes for CFG ---
        let stmt_index = Self::assemble_step7(files, &mut graph, &mut location_index);

        // --- Step 8: ControlFlow edges ---
        for (from, to, w) in Self::collect_step8_edges(&stmt_index, files) {
            graph.add_edge(from, to, w);
        }

        // --- Step 9: Virtual dispatch enrichment ---
        if let Some(ref tdb) = type_db {
            let mut virtual_edges: Vec<(NodeIndex, NodeIndex)> = Vec::new();
            let live_classes = TypeDatabase::collect_live_classes(files);
            // §17: restrict CHA enrichment to type_db-owned (C/C++) functions so a
            // same-named function in another language (e.g. Go `Handle`) can never be
            // minted a cross-language Exact override edge. Sound exact file match on the
            // `RecordInfo.file` set; empty set (records without file info, e.g. synthetic
            // tests) disables the filter rather than excluding everything. NOTE: absolute
            // vs. repo-relative path reconciliation is deferred to PR-2 — exact match is
            // conservative (it can only *drop* edges, never mint a wrong one).
            let owned: BTreeSet<&str> = tdb
                .records
                .values()
                .map(|r| r.file.as_str())
                .filter(|f| !f.is_empty())
                .collect();
            let mut virtual_method_nodes: BTreeMap<String, Vec<(String, NodeIndex)>> =
                BTreeMap::new();
            for record in tdb.records.values() {
                for method_name in record.virtual_methods.keys() {
                    for ((file, name, _start_line), &idx) in &func_index {
                        if name == method_name
                            && (owned.is_empty() || owned.contains(file.as_str()))
                        {
                            virtual_method_nodes
                                .entry(method_name.clone())
                                .or_default()
                                .push((record.name.clone(), idx));
                        }
                    }
                }
            }
            for ((caller_file, _name, _start_line), &caller_idx) in &func_index {
                if !owned.is_empty() && !owned.contains(caller_file.as_str()) {
                    continue;
                }
                let callees: Vec<_> = graph
                    .edges(caller_idx)
                    // EFT: only Exact call edges seed CHA expansion. A NameOnly
                    // edge must not launder into freshly minted Exact CHA edges.
                    .filter(|e| matches!(e.weight(), CpgEdge::Call(ResolutionConfidence::Exact)))
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
                // CHA dispatch is type-confirmed = Exact. Guard only on an
                // existing Exact edge so a NameOnly pair is upgraded.
                let already_exists = graph.edges(*from).any(|e| {
                    e.target() == *to
                        && matches!(e.weight(), CpgEdge::Call(ResolutionConfidence::Exact))
                });
                if !already_exists {
                    graph.add_edge(*from, *to, CpgEdge::Call(ResolutionConfidence::Exact));
                    graph.add_edge(*to, *from, CpgEdge::Return(ResolutionConfidence::Exact));
                }
            }
        }

        for nodes in location_index.values_mut() {
            nodes.sort_by_key(|&i| match &graph[i] {
                CpgNode::Variable {
                    start_byte,
                    end_byte,
                    access,
                    ..
                } => (
                    0u8,
                    *start_byte,
                    *end_byte,
                    match access {
                        VarAccess::Def => 0,
                        VarAccess::Use => 1,
                    },
                    i.index(),
                ),
                _ => (1u8, 0, 0, 0, i.index()),
            });
        }

        CodePropertyGraph {
            graph,
            func_index,
            name_index,
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
    pub(crate) fn assemble_step7(
        files: &BTreeMap<String, ParsedFile>,
        graph: &mut DiGraph<CpgNode, CpgEdge>,
        location_index: &mut BTreeMap<(String, usize), Vec<NodeIndex>>,
    ) -> BTreeMap<(String, usize), NodeIndex> {
        use rayon::prelude::*;

        // 1. Ordered files (BTreeMap order — NOT scheduler order).
        let ordered: Vec<(&String, &ParsedFile)> = files.iter().collect();

        // 2. Parallel collect (read-only).
        let per_file: Vec<(&String, Vec<PendingStatement>)> = ordered
            .par_iter()
            .map(|(path, parsed)| {
                let func_types = parsed.language.function_node_types();
                let mut seen: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
                let mut out: Vec<PendingStatement> = Vec::new();
                Self::collect_pending(
                    parsed.tree.root_node(),
                    &func_types,
                    parsed,
                    &mut seen,
                    &mut out,
                );
                (*path, out)
            })
            .collect(); // rayon indexed collect is order-preserving => ordered-files order

        // 3. Serial create (the ONLY mutation, in files-order x walk-order).
        let mut stmt_index: BTreeMap<(String, usize), NodeIndex> = BTreeMap::new();
        for (path, stmts) in &per_file {
            for s in stmts {
                let idx = graph.add_node(CpgNode::Statement {
                    file: (*path).clone(),
                    line: s.line,
                    kind: s.kind.clone(),
                    start_byte: s.start_byte,
                    end_byte: s.end_byte,
                });
                stmt_index.insert(((*path).clone(), s.line), idx);
                location_index
                    .entry(((*path).clone(), s.line))
                    .or_default()
                    .push(idx);
            }
        }
        stmt_index
    }

    /// Step 8: statement->statement ControlFlow edges. Parallel collect over
    /// ordered file units (`par_iter`, order-preserving), then serial `add_edge`.
    pub(crate) fn collect_step8_edges(
        stmt_index: &BTreeMap<(String, usize), NodeIndex>,
        files: &BTreeMap<String, ParsedFile>,
    ) -> Vec<PendingEdge> {
        use rayon::prelude::*;

        let ordered: Vec<(&String, &ParsedFile)> = files.iter().collect();
        ordered
            .par_iter()
            .map(|(_path, parsed)| {
                let mut out: Vec<PendingEdge> = Vec::new();
                for edge in cfg::build_cfg_edges(parsed) {
                    let from_idx = stmt_index.get(&(edge.file.clone(), edge.from_line));
                    let to_idx = stmt_index.get(&(edge.file.clone(), edge.to_line));
                    if let (Some(&from), Some(&to)) = (from_idx, to_idx) {
                        out.push((from, to, CpgEdge::ControlFlow));
                    }
                }
                out
            })
            .collect::<Vec<Vec<PendingEdge>>>()
            .into_iter()
            .flatten()
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn collect_step8_edges_reference(
        stmt_index: &BTreeMap<(String, usize), NodeIndex>,
        files: &BTreeMap<String, ParsedFile>,
    ) -> Vec<PendingEdge> {
        let mut out: Vec<PendingEdge> = Vec::new();
        for (_path, parsed) in files {
            let cfg_edges = cfg::build_cfg_edges(parsed);
            for edge in cfg_edges {
                let from_idx = stmt_index.get(&(edge.file.clone(), edge.from_line));
                let to_idx = stmt_index.get(&(edge.file.clone(), edge.to_line));
                if let (Some(&from), Some(&to)) = (from_idx, to_idx) {
                    out.push((from, to, CpgEdge::ControlFlow));
                }
            }
        }
        out
    }

    #[cfg(test)]
    pub(crate) fn assemble_step7_reference(
        files: &BTreeMap<String, ParsedFile>,
        graph: &mut DiGraph<CpgNode, CpgEdge>,
        location_index: &mut BTreeMap<(String, usize), Vec<NodeIndex>>,
    ) -> BTreeMap<(String, usize), NodeIndex> {
        let mut stmt_index: BTreeMap<(String, usize), NodeIndex> = BTreeMap::new();
        for (path, parsed) in files {
            let root = parsed.tree.root_node();
            let func_types = parsed.language.function_node_types();
            Self::collect_function_statements(
                root,
                &func_types,
                parsed,
                path,
                graph,
                &mut stmt_index,
                location_index,
            );
        }
        stmt_index
    }

    /// Read-only per-file collect: the `collect_function_statements` recursion with
    /// node creation removed. Whole-file `seen` (by line), checked BEFORE classify -
    /// exactly the current `if stmt_index.contains_key(&(file,line)) { continue }`.
    fn collect_pending(
        node: tree_sitter::Node<'_>,
        func_types: &[&str],
        parsed: &ParsedFile,
        seen: &mut std::collections::BTreeSet<usize>,
        out: &mut Vec<PendingStatement>,
    ) {
        if func_types.contains(&node.kind()) {
            for span in parsed.statement_spans_in_function(&node) {
                if !seen.insert(span.line) {
                    continue; // duplicate (file,line) - skip BEFORE classify, as today
                }
                let kind = Self::classify_stmt_kind(&span.kind, parsed, span.line);
                out.push(PendingStatement {
                    line: span.line,
                    kind,
                    start_byte: span.start_byte,
                    end_byte: span.end_byte,
                });
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::collect_pending(child, func_types, parsed, seen, out);
        }
    }

    #[cfg(test)]
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
            let stmts = parsed.statement_spans_in_function(&node);
            for stmt in stmts {
                let line = stmt.line;
                let key = (file.to_string(), line);
                if stmt_index.contains_key(&key) {
                    continue;
                }
                let kind = Self::classify_stmt_kind(&stmt.kind, parsed, line);
                let idx = graph.add_node(CpgNode::Statement {
                    file: file.to_string(),
                    line,
                    kind,
                    start_byte: stmt.start_byte,
                    end_byte: stmt.end_byte,
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
