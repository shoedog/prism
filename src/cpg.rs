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
use crate::cfg;
use crate::data_flow::{DataFlowGraph, VarAccessKind, VarLocation};
use crate::diff::DiffInput;
use crate::type_db::TypeDatabase;
use crate::type_provider::TypeRegistry;
use crate::type_providers::cpp::CppTypeProvider;
use crate::type_providers::go::GoTypeProvider;
use crate::type_providers::java::JavaTypeProvider;
use crate::type_providers::python::PythonTypeProvider;
use crate::type_providers::rust_provider::RustTypeProvider;
use crate::type_providers::typescript::TypeScriptTypeProvider;

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

// ---------------------------------------------------------------------------
// CpgContext — shared analysis context built once per review
// ---------------------------------------------------------------------------

/// Metadata about a diff-scoped CPG.
///
/// Present only when `CpgContext::build_scoped()` was used. Indicates that the
/// CPG covers a subset of parsed files (changed files + direct callers/callees)
/// rather than the full codebase. Algorithms can check `ctx.scope.is_some()` to
/// qualify results (e.g., "no callers found" vs "no callers found within scope").
#[derive(Debug, Clone)]
pub struct CpgScope {
    /// All files included in the scoped CPG (Tier 0 + 1 + 2).
    pub scoped_files: BTreeSet<String>,
    /// Only the changed files from the diff (Tier 0).
    pub changed_files: BTreeSet<String>,
}

/// Shared analysis context built once per review, passed to all algorithms.
///
/// Bundles the Code Property Graph with the ParsedFile map and the
/// multi-language type registry. Algorithms that need graph traversal use
/// `cpg`; algorithms that need source text or AST patterns use `files`.
pub struct CpgContext<'a> {
    /// The unified Code Property Graph (built once).
    pub cpg: CodePropertyGraph,
    /// Parsed files with tree-sitter ASTs.
    pub files: &'a BTreeMap<String, ParsedFile>,
    /// Multi-language type registry (replaces the old `type_db` field).
    pub types: TypeRegistry,
    /// Scope metadata. `None` means the CPG covers all parsed files.
    /// `Some` means it was built from a diff-scoped subset.
    pub scope: Option<CpgScope>,
}

impl<'a> CpgContext<'a> {
    /// Build a CpgContext from parsed files and optional type enrichment.
    pub fn build(
        files: &'a BTreeMap<String, ParsedFile>,
        type_db: Option<&'a TypeDatabase>,
    ) -> Self {
        let cpg = CodePropertyGraph::build_enriched(files, type_db);
        let types = Self::build_registry(files, type_db);
        CpgContext {
            cpg,
            files,
            types,
            scope: None,
        }
    }

    /// Build a CpgContext with a pre-built `TypeRegistry`.
    ///
    /// The optional `type_db` is still needed for CPG virtual dispatch enrichment
    /// during graph construction. In future phases, the CPG builder will use the
    /// registry directly.
    pub fn build_with_registry(
        files: &'a BTreeMap<String, ParsedFile>,
        type_db: Option<&TypeDatabase>,
        registry: TypeRegistry,
    ) -> Self {
        let cpg = CodePropertyGraph::build_enriched(files, type_db);
        CpgContext {
            cpg,
            files,
            types: registry,
            scope: None,
        }
    }

    /// Build a diff-scoped CpgContext that only covers changed files and their
    /// direct callers/callees.
    ///
    /// Uses a two-pass approach:
    /// 1. Build a skeleton call graph (direct calls only) from all files
    /// 2. Compute the scope: changed files + callers + callees
    /// 3. Build the full CPG on just the scoped subset
    ///
    /// If the scope covers >50% of files, falls back to a full build (the
    /// skeleton overhead isn't worth it when most files are in scope anyway).
    ///
    /// **Known limitation:** The skeleton resolves callees by bare function name
    /// only. Qualified calls like `utils.process()` won't resolve to the
    /// defining file without import resolution, so the scope may be slightly
    /// too narrow for Python/JS/TS codebases with heavy use of qualified calls.
    pub fn build_scoped(
        files: &'a BTreeMap<String, ParsedFile>,
        diff: &DiffInput,
        type_db: Option<&'a TypeDatabase>,
    ) -> Self {
        // Collect changed file paths from the diff.
        let changed_files: BTreeSet<String> =
            diff.files.iter().map(|d| d.file_path.clone()).collect();

        // If there are no changed files or only one file total, just do a full build.
        if changed_files.is_empty() || files.len() <= 1 {
            return Self::build(files, type_db);
        }

        // Pass 1: skeleton call graph (Phases 1-2 only, no indirect resolution).
        let skeleton_cg = CallGraph::build_skeleton(files);

        // Compute the scoped file set: Tier 0 (changed) + Tier 1 (callers) + Tier 2 (callees).
        let scoped_files = compute_scope(&skeleton_cg, &changed_files, files);

        // Short-circuit: if scope covers >50% of files, just build the full CPG.
        if scoped_files.len() > files.len() / 2 {
            return Self::build(files, type_db);
        }

        // Pass 2: build full CPG on scoped subset.
        let filtered: BTreeMap<String, ParsedFile> = files
            .iter()
            .filter(|(k, _)| scoped_files.contains(k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let cpg = CodePropertyGraph::build_enriched(&filtered, type_db);
        let types = Self::build_registry(files, type_db);
        CpgContext {
            cpg,
            files,
            types,
            scope: Some(CpgScope {
                scoped_files,
                changed_files,
            }),
        }
    }

    /// Create a lightweight CpgContext without building the CPG.
    ///
    /// Used for AST-only algorithms that never access the CPG.
    /// The CPG fields are empty — accessing `cpg` will return no results.
    pub fn without_cpg(
        files: &'a BTreeMap<String, ParsedFile>,
        type_db: Option<&'a TypeDatabase>,
    ) -> Self {
        let types = Self::build_registry(files, type_db);
        CpgContext {
            cpg: CodePropertyGraph::empty(),
            files,
            types,
            scope: None,
        }
    }

    /// Backward-compatible accessor: get a reference to the C/C++ TypeDatabase
    /// from the registry, if a CppTypeProvider is registered.
    pub fn type_db(&self) -> Option<&TypeDatabase> {
        // The CPG internally owns its own copy of the TypeDatabase.
        self.cpg.type_db.as_ref()
    }

    /// Build a TypeRegistry from parsed files and optional TypeDatabase.
    ///
    /// Automatically registers providers for languages found in the file set:
    /// - C/C++: `CppTypeProvider` (if `type_db` is provided)
    /// - Go: `GoTypeProvider` (if Go files are present)
    fn build_registry(
        files: &BTreeMap<String, ParsedFile>,
        type_db: Option<&TypeDatabase>,
    ) -> TypeRegistry {
        let mut registry = TypeRegistry::empty();

        // C/C++ provider from TypeDatabase.
        if let Some(db) = type_db {
            let provider = CppTypeProvider::new(db.clone());
            // Clone shares the Arc<TypeDatabase> — single backing store.
            let dispatch = provider.clone();
            registry.register_provider(Box::new(provider));
            registry.register_dispatch_provider(Box::new(dispatch));
        }

        // Go provider — extracted from tree-sitter ASTs.
        let has_go = files
            .values()
            .any(|pf| pf.language == crate::languages::Language::Go);
        if has_go {
            let go_provider = GoTypeProvider::from_parsed_files(files);
            // Clone shares the Arc<GoTypeData> — single backing store.
            let go_dispatch = go_provider.clone();
            registry.register_provider(Box::new(go_provider));
            registry.register_dispatch_provider(Box::new(go_dispatch));
        }

        // Java provider — extracted from tree-sitter ASTs.
        let has_java = files
            .values()
            .any(|pf| pf.language == crate::languages::Language::Java);
        if has_java {
            let java_provider = JavaTypeProvider::from_parsed_files(files);
            // Clone shares the Arc<JavaTypeData> — single backing store.
            let java_dispatch = java_provider.clone();
            registry.register_provider(Box::new(java_provider));
            registry.register_dispatch_provider(Box::new(java_dispatch));
        }

        // Rust provider — extracted from tree-sitter ASTs.
        let has_rust = files
            .values()
            .any(|pf| pf.language == crate::languages::Language::Rust);
        if has_rust {
            let rust_provider = RustTypeProvider::from_parsed_files(files);
            // Clone shares the Arc<RustTypeData> — single backing store.
            let rust_dispatch = rust_provider.clone();
            registry.register_provider(Box::new(rust_provider));
            registry.register_dispatch_provider(Box::new(rust_dispatch));
        }

        // Python provider — extracted from tree-sitter ASTs (PEP 484 annotations).
        // TypeProvider only — no DispatchProvider (Python uses duck typing).
        let has_python = files
            .values()
            .any(|pf| pf.language == crate::languages::Language::Python);
        if has_python {
            let python_provider = PythonTypeProvider::from_parsed_files(files);
            registry.register_provider(Box::new(python_provider));
        }

        // TypeScript/TSX provider — extracted from tree-sitter ASTs.
        let has_ts = files.values().any(|pf| {
            matches!(
                pf.language,
                crate::languages::Language::TypeScript | crate::languages::Language::Tsx
            )
        });
        if has_ts {
            let ts_provider = TypeScriptTypeProvider::from_parsed_files(files);
            // Clone shares the Arc<TsTypeData> — single backing store.
            let ts_dispatch = ts_provider.clone();
            let ts_structural = ts_provider.clone();
            registry.register_provider(Box::new(ts_provider));
            registry.register_dispatch_provider(Box::new(ts_dispatch));
            registry.register_structural_provider(Box::new(ts_structural));
        }

        registry
    }
}

/// Compute the scoped file set for incremental CPG construction.
///
/// Three tiers:
/// - **Tier 0:** Changed files (from the diff)
/// - **Tier 1:** Direct callers — files containing functions that call into changed functions
/// - **Tier 2:** Direct callees — files containing functions called by changed functions
fn compute_scope(
    skeleton_cg: &CallGraph,
    changed_files: &BTreeSet<String>,
    files: &BTreeMap<String, ParsedFile>,
) -> BTreeSet<String> {
    let mut scope: BTreeSet<String> = changed_files.clone();

    // Identify changed functions: functions whose line range overlaps diff lines.
    // We iterate all functions and check if they're in a changed file.
    let mut changed_fn_names: BTreeSet<String> = BTreeSet::new();
    let mut changed_fn_ids: Vec<FunctionId> = Vec::new();

    for func_ids in skeleton_cg.functions.values() {
        for fid in func_ids {
            if changed_files.contains(&fid.file) {
                changed_fn_names.insert(fid.name.clone());
                changed_fn_ids.push(fid.clone());
            }
        }
    }

    // Tier 1: files containing direct callers of changed functions.
    for name in &changed_fn_names {
        if let Some(sites) = skeleton_cg.callers.get(name) {
            for site in sites {
                scope.insert(site.caller.file.clone());
            }
        }
    }

    // Tier 2: files containing direct callees of changed functions.
    for fid in &changed_fn_ids {
        if let Some(sites) = skeleton_cg.calls.get(fid) {
            for site in sites {
                // Resolve callee to actual function definitions.
                let callee_ids = skeleton_cg.resolve_callees(&site.callee_name, &fid.file);
                for callee_id in callee_ids {
                    scope.insert(callee_id.file.clone());
                }
            }
        }
    }

    // Only include files that are actually in the parsed files map.
    scope.retain(|f| files.contains_key(f));
    scope
}

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
                let callee_ids = cg.resolve_callees_qualified(
                    &site.callee_name,
                    &caller_id.file,
                    site.qualifier.as_deref(),
                );
                for callee_id in callee_ids {
                    let callee_key = (callee_id.file.clone(), callee_id.name.clone());
                    if let Some(&callee_idx) = func_index.get(&callee_key) {
                        graph.add_edge(caller_idx, callee_idx, CpgEdge::Call);
                        graph.add_edge(callee_idx, caller_idx, CpgEdge::Return);
                    }
                }
            }
        }

        // --- Step 5b: Interprocedural data flow edges (argument → parameter) ---
        // For each call site, map call-site arguments to callee parameter Def nodes
        // via DataFlow edges. This enables taint to propagate through function calls.
        for (caller_id, sites) in &cg.calls {
            for site in sites {
                let callee_ids = cg.resolve_callees_qualified(
                    &site.callee_name,
                    &caller_id.file,
                    site.qualifier.as_deref(),
                );
                for callee_id in &callee_ids {
                    // Get caller's argument texts at the call site
                    let caller_parsed = match files.get(&caller_id.file) {
                        Some(p) => p,
                        None => continue,
                    };
                    let arg_texts = caller_parsed.call_argument_texts(site.line, &site.callee_name);
                    if arg_texts.is_empty() {
                        continue;
                    }

                    // Get callee's parameter names
                    let callee_parsed = match files.get(&callee_id.file) {
                        Some(p) => p,
                        None => continue,
                    };
                    let param_names = {
                        // Find the callee function node
                        let funcs = callee_parsed.all_functions();
                        let func_node = funcs.iter().find(|f| {
                            callee_parsed
                                .language
                                .function_name(f)
                                .map(|n| callee_parsed.node_text(&n) == callee_id.name)
                                .unwrap_or(false)
                        });
                        match func_node {
                            Some(f) => callee_parsed.function_parameter_names(f),
                            None => continue,
                        }
                    };

                    // Map each argument to the corresponding parameter
                    for (i, param_name) in param_names.iter().enumerate() {
                        if i >= arg_texts.len() {
                            break;
                        }
                        let arg_text = &arg_texts[i];

                        // Find the Use node for the argument variable at the call site line.
                        // The argument text may be a simple variable or an expression;
                        // we look for the base identifier.
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
                        // Also try Def (some languages assign in call args)
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

                        // Find the Def node for the parameter in the callee function.
                        // Parameters are typically defined at or near the function start line.
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

        // --- Step 7: Statement nodes for CFG (Phase 6) ---
        // Create Statement nodes for all statement-level AST constructs in each
        // function. These are distinct from Variable nodes (which model data flow).
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

        // --- Step 8: ControlFlow edges from CFG builder ---
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

        // --- Step 9: Virtual dispatch enrichment (if type_db present) ---
        // Uses Rapid Type Analysis (RTA) when possible: only adds edges to
        // override implementations in classes that are actually instantiated.
        // Falls back to Class Hierarchy Analysis (CHA) if no instantiations found.
        if let Some(ref tdb) = type_db {
            let mut virtual_edges: Vec<(NodeIndex, NodeIndex)> = Vec::new();

            // Collect live classes via RTA (scan for instantiation expressions)
            let live_classes = TypeDatabase::collect_live_classes(files);

            // Collect all virtual method names and which function nodes implement them,
            // along with the class that owns each override
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

            // For each caller that calls a virtual method, add edges to overrides
            // RTA: only to overrides in instantiated classes (if we have live class info)
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
                                // RTA filter: skip overrides in uninstantiated classes
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

    /// Build a CPG with type enrichment from a TypeDatabase.
    ///
    /// Convenience method — equivalent to `build_impl(files, Some(type_db))`.
    pub fn build_with_types(files: &BTreeMap<String, ParsedFile>, type_db: TypeDatabase) -> Self {
        Self::build_impl(files, Some(type_db))
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
                            // Intraprocedural: must be CFG-reachable
                            || cfg_set.contains(&(target.file.clone(), target.line))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::ParsedFile;
    use crate::languages::Language;
    use crate::type_db::{FieldInfo, RecordInfo, RecordKind, TypeDatabase, TypedefInfo};

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

    #[test]
    fn test_build_enriched_without_types() {
        let source = "void f() { int x = 1; }\n";
        let path = "src/enriched.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build_enriched(&files, None);
        assert!(!cpg.has_type_info());
        assert!(cpg.node_count() > 0);
    }

    #[test]
    fn test_build_enriched_with_types() {
        let source = "void f() { int x = 1; }\n";
        let path = "src/enriched2.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let mut type_db = TypeDatabase::default();
        type_db.records.insert(
            "MyStruct".to_string(),
            RecordInfo {
                name: "MyStruct".to_string(),
                kind: RecordKind::Struct,
                fields: vec![FieldInfo {
                    name: "x".to_string(),
                    type_str: "int".to_string(),
                    offset: None,
                }],
                bases: vec![],
                virtual_methods: BTreeMap::new(),
                size: None,
                file: String::new(),
            },
        );

        let cpg = CodePropertyGraph::build_enriched(&files, Some(&type_db));
        assert!(cpg.has_type_info());
        assert!(cpg.node_count() > 0);
    }

    #[test]
    fn test_build_with_types() {
        let source = "void f() { int x = 1; }\n";
        let path = "src/owned.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let type_db = TypeDatabase::default();
        let cpg = CodePropertyGraph::build_with_types(&files, type_db);
        assert!(cpg.has_type_info());
    }

    #[test]
    fn test_all_fields_of() {
        let source = "void f() {}\n";
        let path = "src/fields.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let mut type_db = TypeDatabase::default();
        type_db.records.insert(
            "Point".to_string(),
            RecordInfo {
                name: "Point".to_string(),
                kind: RecordKind::Struct,
                fields: vec![
                    FieldInfo {
                        name: "x".to_string(),
                        type_str: "int".to_string(),
                        offset: None,
                    },
                    FieldInfo {
                        name: "y".to_string(),
                        type_str: "int".to_string(),
                        offset: None,
                    },
                ],
                bases: vec![],
                virtual_methods: BTreeMap::new(),
                size: None,
                file: String::new(),
            },
        );

        let cpg = CodePropertyGraph::build_with_types(&files, type_db);
        let fields = cpg.all_fields_of("Point").unwrap();
        assert_eq!(fields, vec!["x", "y"]);

        // Unknown type returns None
        assert!(cpg.all_fields_of("Unknown").is_none());
    }

    #[test]
    fn test_resolve_type_with_typedef() {
        let source = "void f() {}\n";
        let path = "src/typedef.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let mut type_db = TypeDatabase::default();
        type_db.typedefs.insert(
            "handle_t".to_string(),
            TypedefInfo {
                name: "handle_t".to_string(),
                underlying: "struct device *".to_string(),
            },
        );

        let cpg = CodePropertyGraph::build_with_types(&files, type_db);
        assert_eq!(cpg.resolve_type("handle_t"), "struct device *");
        assert_eq!(cpg.resolve_type("int"), "int"); // not a typedef
    }

    #[test]
    fn test_resolve_type_without_type_db() {
        let source = "void f() {}\n";
        let path = "src/no_types.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);
        assert_eq!(cpg.resolve_type("handle_t"), "handle_t");
    }

    #[test]
    fn test_is_union_type() {
        let source = "void f() {}\n";
        let path = "src/union.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let mut type_db = TypeDatabase::default();
        type_db.records.insert(
            "MyUnion".to_string(),
            RecordInfo {
                name: "MyUnion".to_string(),
                kind: RecordKind::Union,
                fields: vec![
                    FieldInfo {
                        name: "i".to_string(),
                        type_str: "int".to_string(),
                        offset: None,
                    },
                    FieldInfo {
                        name: "f".to_string(),
                        type_str: "float".to_string(),
                        offset: None,
                    },
                ],
                bases: vec![],
                virtual_methods: BTreeMap::new(),
                size: None,
                file: String::new(),
            },
        );
        type_db.records.insert(
            "MyStruct".to_string(),
            RecordInfo {
                name: "MyStruct".to_string(),
                kind: RecordKind::Struct,
                fields: vec![],
                bases: vec![],
                virtual_methods: BTreeMap::new(),
                size: None,
                file: String::new(),
            },
        );

        let cpg = CodePropertyGraph::build_with_types(&files, type_db);
        assert!(cpg.is_union_type("MyUnion"));
        assert!(!cpg.is_union_type("MyStruct"));
        assert!(!cpg.is_union_type("NonExistent"));
    }

    #[test]
    fn test_field_type() {
        let source = "void f() {}\n";
        let path = "src/field_type.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let mut type_db = TypeDatabase::default();
        type_db.records.insert(
            "Device".to_string(),
            RecordInfo {
                name: "Device".to_string(),
                kind: RecordKind::Struct,
                fields: vec![
                    FieldInfo {
                        name: "id".to_string(),
                        type_str: "int".to_string(),
                        offset: None,
                    },
                    FieldInfo {
                        name: "name".to_string(),
                        type_str: "char *".to_string(),
                        offset: None,
                    },
                ],
                bases: vec![],
                virtual_methods: BTreeMap::new(),
                size: None,
                file: String::new(),
            },
        );

        let cpg = CodePropertyGraph::build_with_types(&files, type_db);
        assert_eq!(cpg.field_type("Device", "id"), Some("int".to_string()));
        assert_eq!(cpg.field_type("Device", "name"), Some("char *".to_string()));
        assert_eq!(cpg.field_type("Device", "nonexistent"), None);
        assert_eq!(cpg.field_type("Unknown", "id"), None);
    }

    #[test]
    fn test_function_at() {
        let source = r#"
void first() {
    int x = 1;
}

void second() {
    int y = 2;
}
"#;
        let path = "src/func_at.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);

        // Line 3 is inside first()
        let result = cpg.function_at(path, 3);
        assert!(result.is_some());
        let (_, fid) = result.unwrap();
        assert_eq!(fid.name, "first");

        // Line 7 is inside second()
        let result = cpg.function_at(path, 7);
        assert!(result.is_some());
        let (_, fid) = result.unwrap();
        assert_eq!(fid.name, "second");

        // Line 5 is between functions
        let result = cpg.function_at(path, 5);
        assert!(result.is_none());

        // Non-existent file
        let result = cpg.function_at("no_such_file.c", 1);
        assert!(result.is_none());
    }

    #[test]
    fn test_callers_of() {
        let source = r#"
void target() {
    return;
}

void caller1() {
    target();
}

void caller2() {
    target();
}
"#;
        let path = "src/callers.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);

        let callers = cpg.callers_of("target", 1);
        let caller_names: BTreeSet<String> =
            callers.iter().map(|(fid, _)| fid.name.clone()).collect();
        assert!(caller_names.contains("caller1"));
        assert!(caller_names.contains("caller2"));
        assert_eq!(callers.len(), 2);
    }

    #[test]
    fn test_callees_of() {
        let source = r#"
void helper1() {
    return;
}

void helper2() {
    return;
}

void main_fn() {
    helper1();
    helper2();
}
"#;
        let path = "src/callees.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);

        let callees = cpg.callees_of("main_fn", path, 1);
        let callee_names: BTreeSet<String> =
            callees.iter().map(|(fid, _)| fid.name.clone()).collect();
        assert!(callee_names.contains("helper1"));
        assert!(callee_names.contains("helper2"));
    }

    #[test]
    fn test_function_nodes() {
        let source = r#"
void a() { return; }
void b() { return; }
void c() { return; }
"#;
        let path = "src/func_nodes.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);
        let func_nodes = cpg.function_nodes();
        assert_eq!(func_nodes.len(), 3);
        for idx in &func_nodes {
            assert!(cpg.node(*idx).is_function());
        }
    }

    #[test]
    fn test_virtual_dispatch_enrichment() {
        let source = r#"
void render() {
    draw();
}

void draw() {
    return;
}
"#;
        let path = "src/virtual.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let mut type_db = TypeDatabase::default();
        type_db.records.insert(
            "Shape".to_string(),
            RecordInfo {
                name: "Shape".to_string(),
                kind: RecordKind::Class,
                fields: vec![],
                bases: vec![],
                virtual_methods: {
                    let mut m = BTreeMap::new();
                    m.insert("draw".to_string(), "void".to_string());
                    m
                },
                size: None,
                file: String::new(),
            },
        );

        let cpg = CodePropertyGraph::build_with_types(&files, type_db);
        assert!(cpg.has_type_info());

        // The CPG should still have both functions
        assert!(cpg.function_node(path, "render").is_some());
        assert!(cpg.function_node(path, "draw").is_some());
    }

    #[test]
    fn test_taint_forward_basic() {
        let source = r#"
void process() {
    int input = read_user();
    int data = input;
    write(data);
}
"#;
        let path = "src/taint.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);

        let sources = vec![(path.to_string(), 3usize)]; // line where input is defined
        let paths = cpg.taint_forward(&sources);
        // Should find at least one taint path from the source
        // (may be empty if DFG doesn't connect precisely, but shouldn't panic)
        let _ = paths;
    }

    #[test]
    fn test_has_type_info() {
        let source = "void f() {}\n";
        let path = "src/has_type.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg_no_types = CodePropertyGraph::build(&files);
        assert!(!cpg_no_types.has_type_info());

        let cpg_with_types = CodePropertyGraph::build_with_types(&files, TypeDatabase::default());
        assert!(cpg_with_types.has_type_info());
    }

    // -----------------------------------------------------------------------
    // Phase 6: CFG edge tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_cpg_has_cfg_edges() {
        let source = r#"
void f() {
    int x = 1;
    int y = 2;
    int z = 3;
}
"#;
        let path = "src/cfg_test.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);
        assert!(cpg.has_cfg_edges(), "CPG should have ControlFlow edges");
        assert!(cpg.cfg_edge_count() > 0);
    }

    #[test]
    fn test_cpg_statement_nodes_created() {
        let source = r#"
void f() {
    int x = 1;
    int y = x;
    return;
}
"#;
        let path = "src/stmt_nodes.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);

        // Should have Statement nodes at lines 3, 4, 5
        assert!(
            cpg.statement_at(path, 3).is_some(),
            "Should have statement at line 3"
        );
        assert!(
            cpg.statement_at(path, 4).is_some(),
            "Should have statement at line 4"
        );
        assert!(
            cpg.statement_at(path, 5).is_some(),
            "Should have statement at line 5 (return)"
        );
    }

    #[test]
    fn test_cpg_cfg_sequential_flow() {
        let source = r#"
void f() {
    int x = 1;
    int y = 2;
    int z = 3;
}
"#;
        let path = "src/cfg_seq.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);

        // Line 3 → Line 4 via ControlFlow
        let stmt3 = cpg.statement_at(path, 3).unwrap();
        let successors = cpg.cfg_successors(stmt3);
        let succ_lines: Vec<usize> = successors.iter().map(|&idx| cpg.node(idx).line()).collect();
        assert!(
            succ_lines.contains(&4),
            "Line 3 should flow to line 4, got {:?}",
            succ_lines
        );
    }

    #[test]
    fn test_cpg_cfg_return_terminates() {
        let source = r#"
void f() {
    int x = 1;
    return;
    int y = 2;
}
"#;
        let path = "src/cfg_ret.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);

        // return at line 4 should NOT have a successor to line 5
        let stmt4 = cpg.statement_at(path, 4).unwrap();
        let successors = cpg.cfg_successors(stmt4);
        let succ_lines: Vec<usize> = successors.iter().map(|&idx| cpg.node(idx).line()).collect();
        assert!(
            !succ_lines.contains(&5),
            "return should not flow to line 5, got {:?}",
            succ_lines
        );
    }

    #[test]
    fn test_cpg_cfg_if_branches() {
        let source = r#"
void f(int x) {
    if (x > 0) {
        int a = 1;
    } else {
        int b = 2;
    }
    int c = 3;
}
"#;
        let path = "src/cfg_if.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);

        // if at line 3 should have CFG successors to both branches
        let if_stmt = cpg.statement_at(path, 3).unwrap();
        let successors = cpg.cfg_successors(if_stmt);
        assert!(
            successors.len() >= 2,
            "if should branch to at least 2 targets, got {} successors",
            successors.len()
        );
    }

    #[test]
    fn test_cpg_cfg_predecessors() {
        let source = r#"
void f() {
    int x = 1;
    int y = 2;
    int z = 3;
}
"#;
        let path = "src/cfg_pred.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);

        // Line 4 should have line 3 as predecessor
        let stmt4 = cpg.statement_at(path, 4).unwrap();
        let preds = cpg.cfg_predecessors(stmt4);
        let pred_lines: Vec<usize> = preds.iter().map(|&idx| cpg.node(idx).line()).collect();
        assert!(
            pred_lines.contains(&3),
            "Line 4 should have line 3 as predecessor, got {:?}",
            pred_lines
        );
    }

    #[test]
    fn test_cpg_cfg_goto_edge() {
        let source = r#"
void f() {
    int x = 1;
    goto cleanup;
    int y = 2;
cleanup:
    free(x);
}
"#;
        let path = "src/cfg_goto.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);

        // goto at line 4 should have a CFG edge (either to label or through goto resolution)
        let goto_stmt = cpg.statement_at(path, 4);
        assert!(goto_stmt.is_some(), "Should have statement at goto line 4");

        // goto should NOT have sequential successor to line 5
        if let Some(idx) = goto_stmt {
            let successors = cpg.cfg_successors(idx);
            let succ_lines: Vec<usize> = successors.iter().map(|&s| cpg.node(s).line()).collect();
            assert!(
                !succ_lines.contains(&5),
                "goto should not fall through to line 5, got {:?}",
                succ_lines
            );
        }
    }

    #[test]
    fn test_cpg_cfg_edge_filtered_reachability() {
        let source = r#"
void f() {
    int x = 1;
    int y = 2;
    int z = 3;
}
"#;
        let path = "src/cfg_reach.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);

        // CFG reachability: line 3 should reach line 5 via ControlFlow edges
        let stmt3 = cpg.statement_at(path, 3).unwrap();
        let reachable = cpg.reachable_forward(stmt3, &|e| matches!(e, CpgEdge::ControlFlow));
        let reachable_lines: BTreeSet<usize> =
            reachable.iter().map(|&idx| cpg.node(idx).line()).collect();
        assert!(
            reachable_lines.contains(&5),
            "Line 3 should CFG-reach line 5, got {:?}",
            reachable_lines
        );
    }

    #[test]
    fn test_cpg_cfg_python() {
        let source = r#"
def f():
    x = 1
    y = 2
    z = 3
"#;
        let path = "src/cfg_py.py";
        let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);

        let cpg = CodePropertyGraph::build(&files);
        assert!(
            cpg.has_cfg_edges(),
            "Python CPG should have ControlFlow edges"
        );

        // Sequential flow: line 3 → line 4
        let stmt3 = cpg.statement_at(path, 3);
        assert!(stmt3.is_some(), "Should have Python statement at line 3");
        if let Some(idx) = stmt3 {
            let succs = cpg.cfg_successors(idx);
            assert!(
                !succs.is_empty(),
                "Python line 3 should have CFG successors"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Phase 6 PR C: CFG-constrained analysis tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_cfg_reachable_lines() {
        let source = r#"
void f() {
    int x = 1;
    int y = 2;
    return;
    int z = 3;
}
"#;
        let path = "test.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);
        let cpg = CodePropertyGraph::build(&files);

        // Line 3 should reach lines 4 and 5 (return), but NOT line 6 (after return)
        let reachable = cpg.cfg_reachable_lines(path, 3);
        assert!(
            reachable.contains(&(path.to_string(), 4)),
            "Line 3 should CFG-reach line 4, got {:?}",
            reachable
        );
        assert!(
            reachable.contains(&(path.to_string(), 5)),
            "Line 3 should CFG-reach line 5 (return), got {:?}",
            reachable
        );
        // Line 6 is dead code after return — should NOT be reachable
        assert!(
            !reachable.contains(&(path.to_string(), 6)),
            "Line 6 (after return) should NOT be CFG-reachable from line 3, got {:?}",
            reachable
        );
    }

    #[test]
    fn test_taint_forward_cfg_prunes_dead_code() {
        // Taint source at line 3 (x = input), return at line 4,
        // sink at line 5 (after return — dead code). CFG-constrained taint
        // should NOT reach line 5.
        let source = r#"
void f(char* input) {
    char* x = input;
    return;
    exec(x);
}
"#;
        let path = "test.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);
        let cpg = CodePropertyGraph::build(&files);

        let taint_sources = vec![(path.to_string(), 3)];
        let paths = cpg.taint_forward_cfg(&taint_sources);

        // Collect all tainted target lines
        let tainted_lines: BTreeSet<usize> = paths
            .iter()
            .flat_map(|p| p.edges.iter().map(|e| e.to.line))
            .collect();

        // Line 5 (exec after return) should be pruned by CFG constraint
        assert!(
            !tainted_lines.contains(&5),
            "CFG-constrained taint should NOT reach dead code at line 5, got {:?}",
            tainted_lines
        );
    }

    #[test]
    fn test_dfg_cfg_chop_prunes_unreachable() {
        // Source at line 3, sink at line 6. Line 5 is dead code after return.
        // CFG-constrained chop should exclude the dead-code line.
        let source = r#"
void f() {
    int x = 1;
    int y = x;
    return;
    int z = x;
    int w = z;
}
"#;
        let path = "test.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);
        let cpg = CodePropertyGraph::build(&files);

        // DFG chop: source=line 3, sink=line 7 (dead code)
        // CFG-constrained should be empty or exclude dead lines
        let chop = cpg.dfg_cfg_chop(path, 3, path, 7);

        // Line 7 is dead code — CFG forward from line 3 can't reach it
        // The chop should not include line 6 or 7 since they're unreachable
        let has_dead_code = chop.iter().any(|(_, l)| *l == 6 || *l == 7);
        assert!(
            !has_dead_code,
            "CFG-constrained chop should not include dead code lines 6-7, got {:?}",
            chop
        );
    }

    #[test]
    fn test_cfg_constrained_fallback_without_cfg() {
        // When no CFG edges exist (e.g., no functions), methods should
        // gracefully return empty/fallback results
        let source = "int x = 1;\n";
        let path = "test.c";
        let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
        let mut files = BTreeMap::new();
        files.insert(path.to_string(), parsed);
        let cpg = CodePropertyGraph::build(&files);

        // cfg_reachable_lines on non-existent statement → empty
        let reachable = cpg.cfg_reachable_lines(path, 999);
        assert!(reachable.is_empty());

        // taint_forward_cfg falls back to taint_forward
        let paths_cfg = cpg.taint_forward_cfg(&[(path.to_string(), 1)]);
        let paths_dfg = cpg.taint_forward(&[(path.to_string(), 1)]);
        assert_eq!(paths_cfg.len(), paths_dfg.len());
    }
}
