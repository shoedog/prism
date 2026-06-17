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
use crate::cpg::{CodePropertyGraph, CpgEdge, CpgNode, VarAccess};
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
const CACHE_VERSION: u32 = 11; // bincode ignores serde(default) for new trailing fields.

pub const SKIP_POLICY_VERSION: u32 = 1;

/// The on-disk cache structure.
#[derive(Serialize, Deserialize)]
struct CpgCache {
    /// Cache format version. Invalidate entire cache if mismatched.
    version: u32,
    /// Prism crate version string. Invalidate on upgrade.
    prism_version: String,
    /// Build-time fingerprint of tree-sitter grammar versions.
    grammar_fingerprint: String,
    /// Build git SHA (env `GIT_SHA`). Any rebuild at a new commit invalidates, so a
    /// resolver-behavior change auto-invalidates warm caches without a manual
    /// `CACHE_VERSION` bump (S3-deferred #4).
    git_sha: String,
    /// Version of parser skip-policy behavior included in the cache key.
    skip_policy_version: u32,
    /// Per-file content hashes (SHA-256 hex) at time of cache creation.
    file_hashes: BTreeMap<String, String>,
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
        git_sha: env!("GIT_SHA").to_string(),
        skip_policy_version: SKIP_POLICY_VERSION,
        file_hashes: file_hashes.clone(),
        has_type_db,
        graph: SerializedCpg {
            nodes,
            edges,
            call_graph: cpg.call_graph.clone(),
            dfg: cpg.dfg.clone(),
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

    // Resolver/binary identity: a rebuild at a new commit (or a clean<->dirty flip)
    // invalidates, so a resolver-behavior change auto-invalidates warm nav/CPG caches
    // without a manual CACHE_VERSION bump (S3-deferred #4). Dirty-vs-dirty dev iteration
    // shares a `-dirty` git_sha — that path relies on --no-cache.
    if cache.git_sha != env!("GIT_SHA") {
        eprintln!(
            "Cache git_sha mismatch (cached: {}, current: {}), rebuilding",
            cache.git_sha,
            env!("GIT_SHA")
        );
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

    CodePropertyGraph::from_parts(
        graph,
        func_index,
        name_index,
        var_index,
        location_index,
        ser.call_graph,
        ser.dfg,
    )
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
    fn cache_version_is_11_for_arity() {
        // v11: CallSite arg_count/arg_spread (arity disambiguation).
        assert_eq!(super::CACHE_VERSION, 11);
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
            git_sha: env!("GIT_SHA").into(),
            skip_policy_version: SKIP_POLICY_VERSION,
            file_hashes: hashes.clone(),
            has_type_db: false,
            graph: SerializedCpg {
                nodes: vec![],
                edges: vec![],
                call_graph: CallGraph::empty(),
                dfg: DataFlowGraph::empty(),
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
    fn wrong_git_sha_misses() {
        // A warm cache built at a different prism commit must NOT be served — this is
        // what auto-invalidates a resolver change (Phase-IP etc.) with no CACHE_VERSION bump.
        let dir = tempfile::tempdir().unwrap();
        let hashes: BTreeMap<String, String> = BTreeMap::from([("a.py".into(), "h".into())]);
        let bad = CpgCache {
            version: CACHE_VERSION,
            prism_version: env!("CARGO_PKG_VERSION").into(),
            grammar_fingerprint: env!("GRAMMAR_FINGERPRINT").into(),
            git_sha: "0000000000000000-stale".into(),
            skip_policy_version: SKIP_POLICY_VERSION,
            file_hashes: hashes.clone(),
            has_type_db: false,
            graph: SerializedCpg {
                nodes: vec![],
                edges: vec![],
                call_graph: CallGraph::empty(),
                dfg: DataFlowGraph::empty(),
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
