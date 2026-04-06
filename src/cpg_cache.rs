//! Serialized CPG caching for faster subsequent reviews.
//!
//! Saves and loads the `CodePropertyGraph` (petgraph graph + call graph + DFG)
//! to/from disk using bincode. Per-file content hashes (SHA-256) are stored
//! alongside the graph; on load, all hashes are revalidated and the cache is
//! discarded on any mismatch (Phase 1: all-or-nothing invalidation).
//!
//! The `TypeDatabase` inside `CodePropertyGraph` is NOT cached — it is rebuilt
//! from parsed files on every review.

use crate::access_path::AccessPath;
use crate::call_graph::CallGraph;
use crate::cpg::{CodePropertyGraph, CpgEdge, CpgNode, VarAccess};
use crate::data_flow::DataFlowGraph;

use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Current cache format version. Bump this when the serialized format changes
/// (new fields, different node/edge types, etc.) to force a full rebuild.
const CACHE_VERSION: u32 = 1;

/// The on-disk cache structure.
#[derive(Serialize, Deserialize)]
struct CpgCache {
    /// Cache format version. Invalidate entire cache if mismatched.
    version: u32,
    /// Prism crate version string. Invalidate on upgrade.
    prism_version: String,
    /// Per-file content hashes (SHA-256 hex) at time of cache creation.
    file_hashes: BTreeMap<String, String>,
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
pub fn save_cache(
    cpg: &CodePropertyGraph,
    file_hashes: &BTreeMap<String, String>,
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
        file_hashes: file_hashes.clone(),
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
        created: chrono_free_timestamp(),
    };
    let meta_json = serde_json::to_string_pretty(&meta).unwrap_or_default();
    let _ = fs::write(cache_meta_path(cache_dir), meta_json);

    Ok(())
}

/// Attempt to load a cached CPG from the cache directory.
///
/// Returns `Some(CodePropertyGraph)` if the cache exists, the version matches,
/// and ALL file hashes match the current sources. Returns `None` on any
/// mismatch (triggering a full rebuild by the caller).
pub fn load_cache(
    current_hashes: &BTreeMap<String, String>,
    cache_dir: &Path,
) -> Option<CodePropertyGraph> {
    let bin_path = cache_bin_path(cache_dir);
    let data = fs::read(&bin_path).ok()?;

    let cache: CpgCache = bincode::deserialize(&data).ok()?;

    // Version check.
    if cache.version != CACHE_VERSION {
        eprintln!(
            "Cache version mismatch (cached: {}, current: {}), rebuilding",
            cache.version, CACHE_VERSION
        );
        return None;
    }

    // Prism version check.
    let current_version = env!("CARGO_PKG_VERSION");
    if cache.prism_version != current_version {
        eprintln!(
            "Prism version mismatch (cached: {}, current: {}), rebuilding",
            cache.prism_version, current_version
        );
        return None;
    }

    // File hash check: all files must match exactly.
    if cache.file_hashes != *current_hashes {
        return None;
    }

    // Reconstruct DiGraph from node/edge lists.
    let cpg = reconstruct_cpg(cache.graph);
    Some(cpg)
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
    let mut func_index: BTreeMap<(String, String), NodeIndex> = BTreeMap::new();
    let mut var_index: BTreeMap<(String, String, usize, AccessPath, VarAccess), NodeIndex> =
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
                func_index.insert((file.clone(), name.clone()), idx);
                location_index
                    .entry((file.clone(), *start_line))
                    .or_default()
                    .push(idx);
            }
            CpgNode::Variable {
                path,
                file,
                function,
                line,
                access,
            } => {
                var_index.insert(
                    (file.clone(), function.clone(), *line, path.clone(), *access),
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

    CodePropertyGraph::from_parts(
        graph,
        func_index,
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
}
