//! Serialized CPG caching for faster subsequent reviews.
//!
//! Saves and loads the `CodePropertyGraph` (petgraph graph + call graph + DFG)
//! to/from disk using bincode. Per-file content hashes (SHA-256) are stored
//! alongside the graph; on load, all hashes are revalidated and the cache is
//! used only when every file hash matches (Phase 1: all-or-nothing). When any
//! file has changed, changed files are incrementally rebuilt while unchanged
//! portions of the graph are preserved (Phase 2: incremental update).
//!
//! **Scope:** The cache covers only the files referenced in the current diff
//! (i.e., those in `sources`), not the entire repository. This makes it a
//! per-MR cache: re-running the same diff is a hit, but a different diff
//! touching different files will miss and trigger a rebuild.
//!
//! **TypeDatabase handling (Phase 3):** The `TypeDatabase` itself is NOT
//! serialized — it is rebuilt from parsed files / `compile_commands.json` on
//! every review. However, the CPG graph *does* contain virtual dispatch Call
//! edges that are only added when `type_db` is present (Step 9 of graph
//! assembly). To prevent serving a cached graph with mismatched virtual
//! dispatch edges, the cache records whether `type_db` was present at build
//! time. If the current `type_db` availability differs from the cached state,
//! the cache is invalidated (Miss).

use crate::access_path::AccessPath;
use crate::call_graph::CallGraph;
use crate::cpg::{CodePropertyGraph, CpgEdge, CpgNode, ReturnFlowStats, VarAccess};
use crate::data_flow::DataFlowGraph;

use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Current cache format version. Bump this when the serialized format changes
/// (new fields, different node/edge types, etc.) to force a full rebuild.
///
/// History:
/// - v1: Initial cache format (Phases 1+2)
/// - v2: Added `has_type_db` field for type_db consistency (Phase 3)
/// - v3: grammar_fingerprint + skip_policy_version
/// - v4: CallGraph methods/owner indexes + CallSite.receiver_type (S3)
/// - v5: S2 node bytes + VarLocation identity + CallSite bytes.
/// - v6: EFT CpgEdge::Call/Return carry ResolutionConfidence.
/// - v7: + git_sha in cache key (resolver auto-invalidation).
/// - v8: Phase-IP CallGraph.promoted_aliases + embedding_gaps (Go embedding).
/// - v9: Phase-IP CallGraph.interface_impls + interface telemetry.
/// - v10: PR-2 ReceiverRecovery variants (TypeAssertion/VarDecl/SliceElem).
/// - v11: CallSite arg_count/arg_spread (arity disambiguation).
/// - v12: Rust ScopeGraph stored in CallGraph + CallSite.kind + topology key.
/// - v13: Phase-2a PR-1 receiver-typing indices on CallGraph (method_facts,
///   methods_by_scope, identity_complete, field_types, return_types) — inert.
/// - v14: Phase-2a PR-2 CallSite.receiver_outcome materialization field — inert.
/// - v15: Phase-2a PR-3 CallGraph.extension_methods external receiver index.
/// - v16: Scope-graph precision recovery: ScopeGraph.edition_uniform field +
///   Rust-scoped completeness builds the graph on more repos (changed bytes) +
///   owner-keyed disproof prune (changed resolution behavior).
/// - v17: edition_uniform recomputed as anchoring-class (2015 vs 2018+).
/// - v18: ScopeGraph.crate_deps_by_root — per-consuming-crate in-repo dep map
///   for cross-crate `use` leading-segment resolution (changed bincode layout).
/// - v19: glob re-export member expansion (resolution behavior change).
/// - v20: member-visibility tri-state (glob member-rib continuation — behavior change).
/// - v21: method_class_span for self-receiver same-class narrowing.
/// - v22: method_class_span_ambiguous for fail-open line-id collisions.
/// - v23: wrapper-canonical decorated extraction.
/// - v24: Python/JS/TS typed-receiver recovery behavior.
/// - v25: CallSite.receiver_materialized for poisoned local receiver bindings.
/// - v26: class_bases for inherited-self resolution (Slice 1b).
/// - v27: R4c import-binding resolution (import_bindings/module_bindings/indexed_files).
/// - v28: CallSite.origin for source vs indirect-resolution provenance.
/// - v29: JS/TS R4c export and function-local shadow facts on CallGraph.
/// - v30: clean_class_spans for Python recovered inherited receiver resolution.
/// - v31: cache_build_identity replaces raw git_sha as cache validity key.
/// - v32: P3 R6MultiOwnerCandidate — Python/JS/TS/Tsx unknown-receiver
///   multi-owner collisions resolve via `resolve_call_site` to a capped
///   NameOnly candidate instead of dropping, changing CPG Call/Return edges.
/// - v33: P5 Go func-value callbacks — CallGraph.go_package_basenames plus
///   provider-derived struct/func-field facts (S1) + go_registrations and
///   registration telemetry counters (S2); new
///   ResolutionKind::CallbackRegistration/FuncValueField and
///   DropReason::FuncValueFanout (S3, resolution behavior change).
/// - v34: P7 Python `@property`/`@cached_property` access edges —
///   CallGraph.property_getters / cached_property_getters (S1) +
///   property_accesses + property_access_fanout_skips (S2); new
///   ResolutionKind::PropertyAccess (S3, nav-only — no CPG/DataFlow change).
/// - v35: P4 JS/TS export modeling — CallGraph.js_ts_exported_functions (flat
///   name set) replaced by js_ts_exports (typed JsExportFacts) +
///   js_ts_resolved_exports + two barrel telemetry counters; default imports
///   are now MemberImport bindings and destructured `require` is extracted in
///   `extract_import_bindings` (resolution behavior change: default exports,
///   named-export-list renames, const-arrow exports, CommonJS exports, and
///   depth-2-bounded re-export chains now resolve via R4c `ImportMember`).
/// - v36: P8 Rust macro-argument call extraction — CallSite gains
///   `CallSiteOrigin::MacroArg`; CallGraph gains `macro_arg_facts` (S1/S2
///   telemetry); value calls inside a transparency-allowlisted macro's
///   arguments (`assert!(check(x))`, `vec![f(1), f(2)]`, ...) now mint real
///   CallSite entries with `kind: Call` (resolution behavior change: these
///   sites are new, Exact-capable via the normal Rust ladder).
/// - v37: P9 framework-entry edges — CallGraph gains `framework_entries`
///   (Flask/FastAPI/Express route-registration records) +
///   `framework_entry_unresolved_handlers` telemetry; new
///   `ResolutionKind::FrameworkEntry` (S3, nav-only — no CPG/DataFlow
///   change, mirrors `PropertyAccess`).
/// - v38: P11 Go receiver typing — CallGraph gains `go_return_types` (S1),
///   `go_field_types` (S2), `go_package_vars` (S3),
///   embedded-interface route facts (S4); a new post-merge Go receiver
///   rematerialization pass changes which Go `CallSite`s carry a recovered
///   `receiver_type`/`receiver_recovery` (S1 call-RHS, S2 nested-selector,
///   S3 package var); S4 adds an additive Exact `InterfaceDispatch` route for
///   a struct receiver whose method comes only from a directly embedded
///   in-repo interface; new `ResolutionKind::{ReturnTyped, FieldTyped}`
///   (resolution behavior change: new Go Exact/NameOnly edges).
///   embedded-interface routes are keyed by `GoOwnerIdentity`
///   (package-scoped); Go binding walks are fenced by func_literal lexical
///   scope; S4 gate failure drops instead of falling through.
/// - v39: Go build-profile same-package partitioning. CallGraph gains
///   per-file Go package/build profiles plus P13 telemetry counters; Go
///   same-package free-function and receiver-fact recovery now partition by
///   package clause and build constraints.
/// - v40: glob-anchor-fix — Rust `use super::*`/`crate::*`/`self::*` (an
///   EMPTY-path glob with a meaningful anchor) now resolves through the
///   scope-graph engine instead of poisoning; `ResolutionPolicy` gains
///   `glob_anchor_expands`, consulted at the engine's glob poison gate and
///   `resolve_path_guarded`'s empty-segs branch (resolution behavior change:
///   new Rust caller edges out of nested modules/blocks that relied on an
///   anchor-only glob to bring a bare name into scope).
/// - v41: persisted Step-5b arg→param DataFlow edges now select byte-contained
///   occurrences across multi-line call arguments. The corresponding trace
///   descent-gate check is in-memory only.
/// - v42: positional parameter slots now fail closed instead of compressing
///   unknown bindings; persisted Step-5b edges and synthetic Level-3 callback
///   sites therefore have changed topology.
/// - v43: Go embedded-field extraction now persists pointer embeds (bare `*T`,
///   `*pkg.T`, `*T[X]`) with selector name + raw type; qualified embedded targets
///   and pointer-to-interface embeds fail closed in promotion/S4/S2 (P9; single shipped
///   transition on top of main's v42).
/// - v44: P10 Go owner identities carry package clauses; S2/S4/P5 persist raw
///   declaration snapshots, registration provenance, and partition telemetry.
/// - v45: Go `testdata` source exclusion changes the persisted file/dispatch set;
///   strict go.mod parsing plus the immutable go.mod/go.work/Cargo.toml snapshot
///   and symlink-refused/go.work topology entries change Go identity and cache keys.
/// - v46: effective Go workspace/module/replacement identities change persisted
///   interface-dispatch edges and add module-graph/import-path telemetry.
/// - v47: P17 proven Go concrete receivers route before the legacy bare-interface
///   ladder. Its serialized declaration-kind/import-path evidence and value-
///   rebinding shadow bail plus new-recovery provenance are one PR schema
///   transition (paired with sidecar v16).
/// - v48: alias-aware Go signature identity plus the serialized promoted-selector snapshot.
/// - v49: promoted-selector snapshots shadow methods at equal field depth.
/// - v50: return-flow synthetic nodes/edges plus construction telemetry.
/// - v51: Go receiver-origin prerequisites add serialized dot-import evidence
///   and drop counters and change receiver materialization/resolved topology.
/// - v52: Go package-variable receiver facts carry their defining-file owner.
const CACHE_VERSION: u32 = 52;

pub const SKIP_POLICY_VERSION: u32 = 2;

/// The on-disk cache structure.
#[derive(Serialize, Deserialize)]
struct CpgCache {
    /// Cache format version. Invalidate entire cache if mismatched.
    version: u32,
    /// Prism crate version string. Invalidate on upgrade.
    prism_version: String,
    /// Build-time fingerprint of tree-sitter grammar versions.
    grammar_fingerprint: String,
    /// Binary-input cache identity (env `PRISM_CACHE_BUILD_IDENTITY`).
    /// This intentionally ignores docs/eval-only churn while changing when the
    /// analyzer binary inputs change, including dirty implementation edits.
    cache_build_identity: String,
    /// Build git SHA (env `GIT_SHA`) retained for metadata/debugging.
    git_sha: String,
    /// Version of parser skip-policy behavior included in the cache key.
    skip_policy_version: u32,
    /// Per-file content hashes (SHA-256 hex) at time of cache creation.
    file_hashes: BTreeMap<String, String>,
    /// Shape/config key: source file existence + manifest hashes. A mismatch
    /// forces a full miss rather than an incremental source-content rebuild.
    #[serde(default)]
    topology_key: BTreeMap<String, String>,
    /// Whether the cached CPG was built with a TypeDatabase present.
    /// When true, the graph contains virtual dispatch Call edges (Step 9).
    /// A mismatch between this flag and the current `type_db` availability
    /// forces a cache miss to prevent stale/missing virtual dispatch edges.
    has_type_db: bool,
    /// The serialized CPG graph data.
    graph: SerializedCpg,
}

/// Serializable representation of a `CodePropertyGraph`.
///
/// petgraph's `DiGraph` doesn't implement Serialize directly. We serialize it
/// as a node list + edge list with u32 indices, then reconstruct the DiGraph
/// on load with the same insertion order to preserve NodeIndex stability.
#[derive(Serialize, Deserialize)]
struct SerializedCpg {
    /// All graph nodes in insertion order.
    nodes: Vec<CpgNode>,
    /// All graph edges as (source_index, target_index, edge_data).
    edges: Vec<(u32, u32, CpgEdge)>,
    /// The call graph.
    call_graph: CallGraph,
    /// The data flow graph.
    dfg: DataFlowGraph,
    /// Return-flow construction counters survive cold/cache-hit parity.
    return_flow_stats: ReturnFlowStats,
}

/// Human-readable metadata written alongside the binary cache for debugging.
#[derive(Serialize, Deserialize)]
struct CacheMeta {
    prism_version: String,
    cache_version: u32,
    file_count: usize,
    node_count: usize,
    edge_count: usize,
    cache_size_bytes: u64,
    has_type_db: bool,
    cache_build_identity: String,
    git_sha: String,
    created: String,
}

/// Compute SHA-256 hex digest of a file's contents.
fn sha256_hex(contents: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(contents.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Compute file hashes for all source files in the given map.
pub fn compute_file_hashes(sources: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    sources
        .iter()
        .map(|(path, contents)| (path.clone(), sha256_hex(contents)))
        .collect()
}

pub fn compute_topology_key(
    source_hashes: &BTreeMap<String, String>,
    manifest_hashes: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut key = BTreeMap::new();
    for path in source_hashes.keys() {
        key.insert(format!("source:{path}"), "present".to_string());
    }
    for (path, hash) in manifest_hashes {
        key.insert(format!("manifest:{path}"), hash.clone());
    }
    key
}

pub fn current_cache_build_identity() -> &'static str {
    env!("PRISM_CACHE_BUILD_IDENTITY")
}

pub fn binary_input_dirty() -> bool {
    env!("PRISM_BINARY_INPUT_DIRTY") == "1"
}

/// Path to the binary cache file within the cache directory.
fn cache_bin_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join("cpg-cache.bin")
}

/// Path to the metadata JSON file within the cache directory.
fn cache_meta_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join("cache-meta.json")
}

/// Serialize a `CodePropertyGraph` and write it to the cache directory.
///
/// Writes both `cpg-cache.bin` (bincode) and `cache-meta.json` (human-readable).
/// Uses atomic write (temp file + rename) to prevent corruption from interrupted writes.
///
/// `has_type_db` records whether the CPG was built with a `TypeDatabase`.
/// This flag is checked on load to ensure virtual dispatch edge consistency.
pub fn save_cache(
    cpg: &CodePropertyGraph,
    file_hashes: &BTreeMap<String, String>,
    has_type_db: bool,
    cache_dir: &Path,
) -> io::Result<()> {
    let topology_key = compute_topology_key(file_hashes, &BTreeMap::new());
    save_cache_with_topology(cpg, file_hashes, &topology_key, has_type_db, cache_dir)
}

pub fn save_cache_with_topology(
    cpg: &CodePropertyGraph,
    file_hashes: &BTreeMap<String, String>,
    topology_key: &BTreeMap<String, String>,
    has_type_db: bool,
    cache_dir: &Path,
) -> io::Result<()> {
    fs::create_dir_all(cache_dir)?;

    // Extract node list + edge list from DiGraph.
    let nodes: Vec<CpgNode> = cpg
        .graph
        .node_indices()
        .map(|idx| cpg.graph[idx].clone())
        .collect();

    let edges: Vec<(u32, u32, CpgEdge)> = cpg
        .graph
        .edge_indices()
        .map(|eidx| {
            let (src, tgt) = cpg.graph.edge_endpoints(eidx).unwrap();
            (
                src.index() as u32,
                tgt.index() as u32,
                cpg.graph[eidx].clone(),
            )
        })
        .collect();

    let cache = CpgCache {
        version: CACHE_VERSION,
        prism_version: env!("CARGO_PKG_VERSION").to_string(),
        grammar_fingerprint: env!("GRAMMAR_FINGERPRINT").to_string(),
        cache_build_identity: current_cache_build_identity().to_string(),
        git_sha: env!("GIT_SHA").to_string(),
        skip_policy_version: SKIP_POLICY_VERSION,
        file_hashes: file_hashes.clone(),
        topology_key: topology_key.clone(),
        has_type_db,
        graph: SerializedCpg {
            nodes,
            edges,
            call_graph: cpg.call_graph.clone(),
            dfg: cpg.dfg.clone(),
            return_flow_stats: cpg.return_flow_stats.clone(),
        },
    };

    let encoded = bincode::serialize(&cache).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("bincode serialize error: {}", e),
        )
    })?;

    // Atomic write: write to temp file, then rename.
    let bin_path = cache_bin_path(cache_dir);
    let tmp_path = cache_dir.join("cpg-cache.bin.tmp");
    fs::write(&tmp_path, &encoded)?;
    fs::rename(&tmp_path, &bin_path)?;

    // Write human-readable metadata (best-effort, non-fatal).
    let meta = CacheMeta {
        prism_version: env!("CARGO_PKG_VERSION").to_string(),
        cache_version: CACHE_VERSION,
        file_count: file_hashes.len(),
        node_count: cache.graph.nodes.len(),
        edge_count: cache.graph.edges.len(),
        cache_size_bytes: encoded.len() as u64,
        has_type_db,
        cache_build_identity: cache.cache_build_identity.clone(),
        git_sha: cache.git_sha.clone(),
        created: chrono_free_timestamp(),
    };
    let meta_json = serde_json::to_string_pretty(&meta).unwrap_or_default();
    let _ = fs::write(cache_meta_path(cache_dir), meta_json);

    Ok(())
}

/// Result of attempting to load a CPG from cache.
pub enum CacheResult {
    /// All file hashes match — the cached CPG is fully valid.
    Hit(CodePropertyGraph),
    /// Some files match, some differ. The cached CG/DFG for unchanged files
    /// can be retained and merged with fresh analysis of changed files.
    PartialHit {
        /// The cached call graph (to be pruned and merged).
        cached_call_graph: CallGraph,
        /// The cached data flow graph (to be pruned and merged).
        cached_dfg: DataFlowGraph,
        /// Files whose hashes differ from the cache.
        changed_files: BTreeSet<String>,
    },
    /// No usable cache (missing, corrupt, version mismatch, or entirely
    /// different file set).
    Miss,
}

/// Attempt to load a cached CPG from the cache directory.
///
/// Returns `CacheResult::Hit` if all hashes match, `PartialHit` if the file
/// set is the same but some files changed, or `Miss` if the cache is unusable.
///
/// `has_type_db` indicates whether the caller currently has a `TypeDatabase`.
/// If this differs from the cached state, the cache is invalidated because
/// the graph's virtual dispatch edges would be inconsistent.
pub fn load_cache(
    current_hashes: &BTreeMap<String, String>,
    has_type_db: bool,
    cache_dir: &Path,
) -> CacheResult {
    let topology_key = compute_topology_key(current_hashes, &BTreeMap::new());
    load_cache_with_topology(current_hashes, &topology_key, has_type_db, cache_dir)
}

pub fn load_cache_with_topology(
    current_hashes: &BTreeMap<String, String>,
    current_topology_key: &BTreeMap<String, String>,
    has_type_db: bool,
    cache_dir: &Path,
) -> CacheResult {
    let bin_path = cache_bin_path(cache_dir);
    let data = match fs::read(&bin_path) {
        Ok(d) => d,
        Err(_) => return CacheResult::Miss,
    };

    let cache: CpgCache = match bincode::deserialize(&data) {
        Ok(c) => c,
        Err(_) => return CacheResult::Miss,
    };

    // Version check.
    if cache.version != CACHE_VERSION {
        eprintln!(
            "Cache version mismatch (cached: {}, current: {}), rebuilding",
            cache.version, CACHE_VERSION
        );
        return CacheResult::Miss;
    }

    // Prism version check.
    let current_version = env!("CARGO_PKG_VERSION");
    if cache.prism_version != current_version {
        eprintln!(
            "Prism version mismatch (cached: {}, current: {}), rebuilding",
            cache.prism_version, current_version
        );
        return CacheResult::Miss;
    }

    // TypeDatabase consistency check.
    // The cached graph contains virtual dispatch edges only if type_db was
    // present at build time. A mismatch means the graph edges are wrong.
    if cache.has_type_db != has_type_db {
        eprintln!(
            "TypeDatabase mismatch (cached: {}, current: {}), rebuilding",
            cache.has_type_db, has_type_db
        );
        return CacheResult::Miss;
    }

    // Grammar / skip-policy consistency check.
    if cache.grammar_fingerprint != env!("GRAMMAR_FINGERPRINT")
        || cache.skip_policy_version != SKIP_POLICY_VERSION
    {
        return CacheResult::Miss;
    }

    // Resolver/binary identity: cache validity is tied to the analyzer's binary
    // inputs, not repo-wide docs/eval churn. Raw git_sha is metadata only.
    if cache.cache_build_identity != current_cache_build_identity() {
        eprintln!(
            "Cache build identity mismatch (cached: {}, current: {}), rebuilding",
            cache.cache_build_identity,
            current_cache_build_identity()
        );
        return CacheResult::Miss;
    }

    if cache.topology_key != *current_topology_key {
        return CacheResult::Miss;
    }

    // File hash check.
    if cache.file_hashes == *current_hashes {
        // Exact match — full cache hit.
        let cpg = reconstruct_cpg(cache.graph);
        return CacheResult::Hit(cpg);
    }

    // Check for partial match: same file set but some hashes differ.
    // Both maps must have the same keys (same file set).
    let cached_keys: BTreeSet<&String> = cache.file_hashes.keys().collect();
    let current_keys: BTreeSet<&String> = current_hashes.keys().collect();

    if cached_keys != current_keys {
        // Different file sets — can't do incremental.
        return CacheResult::Miss;
    }

    // Same file set, some hashes differ. Identify changed files.
    let changed_files: BTreeSet<String> = current_hashes
        .iter()
        .filter(|(path, hash)| cache.file_hashes.get(*path) != Some(hash))
        .map(|(path, _)| path.clone())
        .collect();

    if changed_files.is_empty() {
        // Shouldn't happen (we checked equality above), but handle gracefully.
        let cpg = reconstruct_cpg(cache.graph);
        return CacheResult::Hit(cpg);
    }

    CacheResult::PartialHit {
        cached_call_graph: cache.graph.call_graph,
        cached_dfg: cache.graph.dfg,
        changed_files,
    }
}

/// Reconstruct a `CodePropertyGraph` from serialized node/edge lists.
fn reconstruct_cpg(ser: SerializedCpg) -> CodePropertyGraph {
    let mut graph = DiGraph::new();

    // Add nodes in order — this preserves NodeIndex values (0, 1, 2, ...).
    for node in &ser.nodes {
        graph.add_node(node.clone());
    }

    // Add edges using the stored indices.
    for &(src, tgt, ref edge) in &ser.edges {
        graph.add_edge(
            NodeIndex::new(src as usize),
            NodeIndex::new(tgt as usize),
            edge.clone(),
        );
    }

    // Rebuild indexes by iterating nodes.
    let mut func_index: BTreeMap<(String, String, usize), NodeIndex> = BTreeMap::new();
    let mut name_index: BTreeMap<(String, String), Vec<NodeIndex>> = BTreeMap::new();
    let mut var_index: BTreeMap<(String, String, usize, usize, AccessPath, VarAccess), NodeIndex> =
        BTreeMap::new();
    let mut location_index: BTreeMap<(String, usize), Vec<NodeIndex>> = BTreeMap::new();

    for idx in graph.node_indices() {
        let node = &graph[idx];
        match node {
            CpgNode::Function {
                name,
                file,
                start_line,
                ..
            } => {
                func_index.insert((file.clone(), name.clone(), *start_line), idx);
                name_index
                    .entry((file.clone(), name.clone()))
                    .or_default()
                    .push(idx);
                location_index
                    .entry((file.clone(), *start_line))
                    .or_default()
                    .push(idx);
            }
            CpgNode::Variable {
                path,
                file,
                function,
                function_start_line,
                line,
                access,
                ..
            } => {
                var_index.insert(
                    (
                        file.clone(),
                        function.clone(),
                        *function_start_line,
                        *line,
                        path.clone(),
                        *access,
                    ),
                    idx,
                );
                location_index
                    .entry((file.clone(), *line))
                    .or_default()
                    .push(idx);
            }
            CpgNode::Statement { file, line, .. } => {
                location_index
                    .entry((file.clone(), *line))
                    .or_default()
                    .push(idx);
            }
            CpgNode::ReturnValue { file, line, .. } => {
                location_index
                    .entry((file.clone(), *line))
                    .or_default()
                    .push(idx);
            }
        }
    }
    for nodes in name_index.values_mut() {
        nodes.sort_by_key(|&n| match &graph[n] {
            CpgNode::Function { start_line, .. } => *start_line,
            _ => usize::MAX,
        });
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

    let mut cpg = CodePropertyGraph::from_parts(
        graph,
        func_index,
        name_index,
        var_index,
        location_index,
        ser.call_graph,
        ser.dfg,
    );
    cpg.return_flow_stats = ser.return_flow_stats;
    cpg
}

/// Simple timestamp without requiring the chrono crate.
fn chrono_free_timestamp() -> String {
    // Use a simple epoch-based approach.
    use std::time::SystemTime;
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => format!("unix:{}", d.as_secs()),
        Err(_) => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_py(path: &str, source: &str) -> crate::ast::ParsedFile {
        crate::ast::ParsedFile::parse(path, source, crate::languages::Language::Python).unwrap()
    }

    fn force_cache_version(cache_dir: &Path, version: u32) {
        let bin_path = cache_bin_path(cache_dir);
        let data = std::fs::read(&bin_path).unwrap();
        let mut cache: CpgCache = bincode::deserialize(&data).unwrap();
        cache.version = version;
        std::fs::write(bin_path, bincode::serialize(&cache).unwrap()).unwrap();
    }

    fn node_dump(cpg: &crate::cpg::CodePropertyGraph) -> Vec<String> {
        cpg.node_indices()
            .map(|idx| format!("{:?}", cpg.node(idx)))
            .collect()
    }

    fn call_site_dump(cpg: &crate::cpg::CodePropertyGraph) -> Vec<String> {
        cpg.call_graph
            .calls
            .values()
            .flat_map(|sites| sites.iter())
            .map(|site| format!("{:?}", site))
            .collect()
    }

    fn cache_result_kind(result: &CacheResult) -> &'static str {
        match result {
            CacheResult::Hit(_) => "Hit",
            CacheResult::PartialHit { .. } => "PartialHit",
            CacheResult::Miss => "Miss",
        }
    }

    #[test]
    fn cache_versions_are_pinned_for_go_receiver_owner_carrying() {
        assert_eq!(super::CACHE_VERSION, 52);
        assert_eq!(super::SKIP_POLICY_VERSION, 2);
    }

    #[test]
    fn test_sha256_hex() {
        let hash = sha256_hex("hello world");
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex chars
                                    // Known hash for "hello world"
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_compute_file_hashes() {
        let mut sources = BTreeMap::new();
        sources.insert("a.py".to_string(), "x = 1".to_string());
        sources.insert("b.py".to_string(), "y = 2".to_string());

        let hashes = compute_file_hashes(&sources);
        assert_eq!(hashes.len(), 2);
        assert!(hashes.contains_key("a.py"));
        assert!(hashes.contains_key("b.py"));
        // Different content → different hash
        assert_ne!(hashes["a.py"], hashes["b.py"]);
    }

    #[test]
    fn v5_round_trip_preserves_byte_identity() {
        let dir = tempfile::tempdir().unwrap();
        let mut sources = BTreeMap::new();
        sources.insert(
            "t.py".to_string(),
            "class C:\n    def m(self, p):\n        q = p\n        return q\n\ndef g(c):\n    return c.m(1)\n"
                .to_string(),
        );
        let files: BTreeMap<String, crate::ast::ParsedFile> = sources
            .iter()
            .map(|(path, source)| (path.clone(), parse_py(path, source)))
            .collect();

        let cpg = crate::cpg::CodePropertyGraph::build(&files);
        let hashes = compute_file_hashes(&sources);

        assert!(
            cpg.node_indices().any(|idx| matches!(
                cpg.node(idx),
                crate::cpg::CpgNode::Variable {
                    file,
                    function_start_line,
                    start_byte,
                    end_byte,
                    ..
                } if file == "t.py"
                    && *function_start_line > 0
                    && *start_byte > 0
                    && *end_byte > *start_byte
            )),
            "fixture should contain a variable node with real byte identity"
        );
        assert!(
            cpg.call_graph
                .calls
                .values()
                .flat_map(|sites| sites.iter())
                .any(|site| site.start_byte > 0 && site.end_byte > site.start_byte),
            "fixture should contain a call site with real byte identity"
        );

        save_cache(&cpg, &hashes, false, dir.path()).unwrap();

        match load_cache(&hashes, false, dir.path()) {
            CacheResult::Hit(restored) => {
                assert_eq!(
                    node_dump(&cpg),
                    node_dump(&restored),
                    "node byte fields and function_start_line should survive round trip"
                );
                assert_eq!(
                    call_site_dump(&cpg),
                    call_site_dump(&restored),
                    "CallSite byte fields should survive round trip"
                );
            }
            other => panic!("expected Hit, got {}", cache_result_kind(&other)),
        }
    }

    #[test]
    fn v4_cache_is_a_miss() {
        let dir = tempfile::tempdir().unwrap();
        let sources = BTreeMap::new();
        let cpg = crate::cpg::CodePropertyGraph::empty();
        let hashes = compute_file_hashes(&sources);

        save_cache(&cpg, &hashes, false, dir.path()).unwrap();
        force_cache_version(dir.path(), 4);

        assert!(matches!(
            load_cache(&hashes, false, dir.path()),
            CacheResult::Miss
        ));
    }

    #[test]
    fn partial_hit_incremental_rebuild_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let mut sources = BTreeMap::new();
        sources.insert("a.py".to_string(), "def a(p):\n    return p\n".to_string());
        sources.insert("b.py".to_string(), "def b(q):\n    return q\n".to_string());
        let mut files: BTreeMap<String, crate::ast::ParsedFile> = sources
            .iter()
            .map(|(path, source)| (path.clone(), parse_py(path, source)))
            .collect();

        let cpg = crate::cpg::CodePropertyGraph::build(&files);
        let h0 = compute_file_hashes(&sources);
        save_cache(&cpg, &h0, false, dir.path()).unwrap();

        sources.insert(
            "b.py".to_string(),
            "def b(q):\n    r = q\n    return r\n".to_string(),
        );
        files.insert("b.py".to_string(), parse_py("b.py", &sources["b.py"]));
        let h1 = compute_file_hashes(&sources);

        match load_cache(&h1, false, dir.path()) {
            CacheResult::PartialHit {
                cached_call_graph,
                cached_dfg,
                changed_files,
            } => {
                assert_eq!(changed_files, BTreeSet::from(["b.py".to_string()]));
                let rebuilt = crate::cpg::CodePropertyGraph::build_incremental(
                    cached_call_graph,
                    cached_dfg,
                    &changed_files,
                    &files,
                    None,
                );
                assert!(
                    rebuilt.node_indices().any(|idx| matches!(
                        rebuilt.node(idx),
                        crate::cpg::CpgNode::Variable {
                            file,
                            start_byte,
                            end_byte,
                            ..
                        } if file == "a.py" && *start_byte > 0 && *end_byte > *start_byte
                    )),
                    "a.py variable nodes should retain real byte spans through incremental rebuild"
                );
            }
            other => panic!("expected PartialHit, got {}", cache_result_kind(&other)),
        }
    }

    #[test]
    fn wrong_grammar_fingerprint_misses() {
        let dir = tempfile::tempdir().unwrap();
        let hashes: BTreeMap<String, String> = BTreeMap::from([("a.py".into(), "h".into())]);
        let bad = CpgCache {
            version: CACHE_VERSION,
            prism_version: env!("CARGO_PKG_VERSION").into(),
            grammar_fingerprint: "deadbeef".into(),
            cache_build_identity: current_cache_build_identity().into(),
            git_sha: env!("GIT_SHA").into(),
            skip_policy_version: SKIP_POLICY_VERSION,
            file_hashes: hashes.clone(),
            topology_key: compute_topology_key(&hashes, &BTreeMap::new()),
            has_type_db: false,
            graph: SerializedCpg {
                nodes: vec![],
                edges: vec![],
                call_graph: CallGraph::empty(),
                dfg: DataFlowGraph::empty(),
                return_flow_stats: ReturnFlowStats::default(),
            },
        };
        std::fs::write(
            dir.path().join("cpg-cache.bin"),
            bincode::serialize(&bad).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            load_cache(&hashes, false, dir.path()),
            CacheResult::Miss
        ));
    }

    #[test]
    fn wrong_cache_build_identity_misses() {
        // A warm cache built from different analyzer binary inputs must NOT be served.
        let dir = tempfile::tempdir().unwrap();
        let hashes: BTreeMap<String, String> = BTreeMap::from([("a.py".into(), "h".into())]);
        let bad = CpgCache {
            version: CACHE_VERSION,
            prism_version: env!("CARGO_PKG_VERSION").into(),
            grammar_fingerprint: env!("GRAMMAR_FINGERPRINT").into(),
            cache_build_identity: "stale-binary-inputs".into(),
            git_sha: env!("GIT_SHA").into(),
            skip_policy_version: SKIP_POLICY_VERSION,
            file_hashes: hashes.clone(),
            topology_key: compute_topology_key(&hashes, &BTreeMap::new()),
            has_type_db: false,
            graph: SerializedCpg {
                nodes: vec![],
                edges: vec![],
                call_graph: CallGraph::empty(),
                dfg: DataFlowGraph::empty(),
                return_flow_stats: ReturnFlowStats::default(),
            },
        };
        std::fs::write(
            dir.path().join("cpg-cache.bin"),
            bincode::serialize(&bad).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            load_cache(&hashes, false, dir.path()),
            CacheResult::Miss
        ));
    }
}
