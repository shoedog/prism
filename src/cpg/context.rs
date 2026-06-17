//! `CpgContext` — the shared analysis context built once per review, plus the
//! diff-scope computation used to build diff-scoped CPGs.

use crate::ast::ParsedFile;
use crate::call_graph::{CallGraph, FunctionId, ScopeGraphBuildInputs};
use crate::diff::DiffInput;
use crate::type_db::TypeDatabase;
use crate::type_provider::TypeRegistry;
use crate::type_providers::cpp::CppTypeProvider;
use crate::type_providers::go::GoTypeProvider;
use crate::type_providers::java::JavaTypeProvider;
use crate::type_providers::python::PythonTypeProvider;
use crate::type_providers::rust_provider::RustTypeProvider;
use crate::type_providers::typescript::TypeScriptTypeProvider;

use std::collections::{BTreeMap, BTreeSet};

use super::CodePropertyGraph;

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
    /// Live (instantiated) types collected via Rapid Type Analysis (RTA).
    ///
    /// Contains type names observed as instantiated across all languages in
    /// the parsed file set. Algorithms can pass this to `DispatchProvider::
    /// resolve_dispatch()` to prune dispatch targets to only live types.
    pub live_types: BTreeSet<String>,
}

impl<'a> CpgContext<'a> {
    /// Build a CpgContext from parsed files and optional type enrichment.
    pub fn build(
        files: &'a BTreeMap<String, ParsedFile>,
        type_db: Option<&'a TypeDatabase>,
    ) -> Self {
        let cpg = CodePropertyGraph::build_enriched(files, type_db);
        Self::from_built_cpg(files, cpg, type_db, None)
    }

    pub fn build_with_scope_graph_inputs(
        files: &'a BTreeMap<String, ParsedFile>,
        type_db: Option<&'a TypeDatabase>,
        scope_inputs: Option<&ScopeGraphBuildInputs>,
    ) -> Self {
        let cpg =
            CodePropertyGraph::build_enriched_with_scope_graph_inputs(files, type_db, scope_inputs);
        Self::from_built_cpg(files, cpg, type_db, None)
    }

    fn from_built_cpg(
        files: &'a BTreeMap<String, ParsedFile>,
        cpg: CodePropertyGraph,
        type_db: Option<&'a TypeDatabase>,
        scope: Option<CpgScope>,
    ) -> Self {
        let types = Self::build_registry(files, type_db);
        let live_types = types.collect_live_types(files);
        CpgContext {
            cpg,
            files,
            types,
            scope,
            live_types,
        }
    }

    /// Build a CpgContext from a cached (deserialized) CPG.
    ///
    /// The CPG graph, call graph, and DFG come from the cache. The type
    /// registry and live types are rebuilt fresh from the parsed files.
    /// If `type_db` is provided (via `--compile-commands`), it is injected
    /// into the cached CPG so that algorithms like `delta_slice` that call
    /// `ctx.cpg.type_db` get virtual dispatch enrichment.
    pub fn build_with_cached_cpg(
        files: &'a BTreeMap<String, ParsedFile>,
        mut cpg: CodePropertyGraph,
        type_db: Option<&'a TypeDatabase>,
    ) -> Self {
        cpg.type_db = type_db.cloned();
        let types = Self::build_registry(files, type_db);
        let live_types = types.collect_live_types(files);
        CpgContext {
            cpg,
            files,
            types,
            scope: None,
            live_types,
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
        let live_types = registry.collect_live_types(files);
        CpgContext {
            cpg,
            files,
            types: registry,
            scope: None,
            live_types,
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

        let cpg = CodePropertyGraph::build_enriched_without_scope_graph(&filtered, type_db);
        let types = Self::build_registry(files, type_db);
        // Collect live types from ALL files (not just scoped) for accurate RTA.
        let live_types = types.collect_live_types(files);
        CpgContext {
            cpg,
            files,
            types,
            scope: Some(CpgScope {
                scoped_files,
                changed_files,
            }),
            live_types,
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
        let live_types = types.collect_live_types(files);
        CpgContext {
            cpg: CodePropertyGraph::empty(),
            files,
            types,
            scope: None,
            live_types,
        }
    }

    /// Get the parse quality grade for a file.
    ///
    /// Returns "clean" (<1%), "degraded" (1-10%), "poor" (10-30%), or "unparseable" (>30%)
    /// based on the fraction of ERROR/MISSING nodes in the tree-sitter parse tree.
    pub fn file_parse_quality(&self, file: &str) -> Option<&str> {
        self.files.get(file).map(|pf| {
            let rate = pf.error_rate();
            if rate > 0.3 {
                "unparseable"
            } else if rate > 0.1 {
                "poor"
            } else if rate > 0.01 {
                "degraded"
            } else {
                "clean"
            }
        })
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
                // S3 contract: scope computation deliberately uses the
                // recall-biased name-only resolver — scope is a superset
                // heuristic, not a truth claim (spec §3.4). Edge creation
                // (cpg/build.rs Step 5) uses the precision ladder.
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
