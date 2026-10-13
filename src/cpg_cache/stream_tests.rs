use super::super::{
    cache_bin_path, cache_meta_path, compute_file_hashes, compute_topology_key,
    current_cache_build_identity, load_cache, save_cache, CacheResult, CpgCache, SerializedCpg,
    CACHE_VERSION, SKIP_POLICY_VERSION,
};
use crate::cpg::{CodePropertyGraph, CpgEdge, CpgNode};
use std::collections::BTreeMap;
use std::fs;

const SOURCE: &str = "class C:\n    def m(self, p):\n        q = p\n        return q\n\ndef g(c):\n    r = c.m(1)\n    return r\n\ndef h(x):\n    return g(x)\n";

fn fixture() -> (CodePropertyGraph, BTreeMap<String, String>) {
    let mut sources = BTreeMap::new();
    sources.insert("t.py".to_string(), SOURCE.to_string());
    let files: BTreeMap<String, crate::ast::ParsedFile> = sources
        .iter()
        .map(|(path, source)| {
            let parsed =
                crate::ast::ParsedFile::parse(path, source, crate::languages::Language::Python)
                    .unwrap();
            (path.clone(), parsed)
        })
        .collect();
    let cpg = CodePropertyGraph::build(&files);
    assert!(cpg.graph.node_count() > 5, "fixture needs nodes");
    assert!(cpg.graph.edge_count() > 3, "fixture needs edges");
    assert!(!cpg.call_graph.calls.is_empty(), "fixture needs call sites");
    (cpg, compute_file_hashes(&sources))
}

/// The encoding the cache used before it streamed: an owned copy of everything, encoded in
/// one buffer. The streamed file must equal it byte for byte, or caches written by one build
/// would not load in the other and `CACHE_VERSION` would have to change.
fn owned_encoding(cpg: &CodePropertyGraph, hashes: &BTreeMap<String, String>) -> Vec<u8> {
    let nodes: Vec<CpgNode> = cpg
        .graph
        .node_indices()
        .map(|index| cpg.graph[index].clone())
        .collect();
    let edges: Vec<(u32, u32, CpgEdge)> = cpg
        .graph
        .edge_indices()
        .map(|index| {
            let (source, target) = cpg.graph.edge_endpoints(index).unwrap();
            (
                source.index() as u32,
                target.index() as u32,
                cpg.graph[index].clone(),
            )
        })
        .collect();
    let owned = CpgCache {
        version: CACHE_VERSION,
        prism_version: env!("CARGO_PKG_VERSION").to_string(),
        grammar_fingerprint: env!("GRAMMAR_FINGERPRINT").to_string(),
        cache_build_identity: current_cache_build_identity().to_string(),
        git_sha: env!("GIT_SHA").to_string(),
        skip_policy_version: SKIP_POLICY_VERSION,
        file_hashes: hashes.clone(),
        topology_key: compute_topology_key(hashes, &BTreeMap::new()),
        has_type_db: false,
        graph: SerializedCpg {
            nodes,
            edges,
            call_graph: cpg.call_graph.clone(),
            dfg: cpg.dfg.clone(),
            return_flow_stats: cpg.return_flow_stats.clone(),
            dfg_label_stats: cpg.dfg_label_stats.clone(),
        },
    };
    bincode::serialize(&owned).unwrap()
}

fn graph_dump(cpg: &CodePropertyGraph) -> (Vec<String>, Vec<String>) {
    let nodes = cpg
        .graph
        .node_indices()
        .map(|index| format!("{}:{:?}", index.index(), cpg.graph[index]))
        .collect();
    let edges = cpg
        .graph
        .edge_indices()
        .map(|index| {
            let (source, target) = cpg.graph.edge_endpoints(index).unwrap();
            format!(
                "{}:{}->{}:{:?}",
                index.index(),
                source.index(),
                target.index(),
                cpg.graph[index]
            )
        })
        .collect();
    (nodes, edges)
}

#[test]
fn the_streamed_file_equals_the_owned_encoding_byte_for_byte() {
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    let written = fs::read(cache_bin_path(dir.path())).unwrap();
    let expected = owned_encoding(&cpg, &hashes);
    assert_eq!(written.len(), expected.len(), "length");
    assert!(
        written == expected,
        "the streamed bytes differ from the owned encoding"
    );
}

#[test]
fn a_file_in_the_owned_encoding_loads_as_a_hit() {
    // A cache written before this change (same bytes, written in one piece) still loads.
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    fs::write(cache_bin_path(dir.path()), owned_encoding(&cpg, &hashes)).unwrap();
    let CacheResult::Hit(restored) = load_cache(&hashes, false, dir.path()) else {
        panic!("a cache in the owned encoding must be a hit");
    };
    assert_eq!(graph_dump(&cpg), graph_dump(&restored));
}

#[test]
fn a_round_trip_keeps_every_node_and_edge_at_its_index() {
    // Loading moves nodes and edges into the graph in list order; an index that shifted would
    // silently re-point every stored NodeIndex.
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(restored) = load_cache(&hashes, false, dir.path()) else {
        panic!("expected a hit");
    };
    assert_eq!(graph_dump(&cpg), graph_dump(&restored));
    assert_eq!(
        format!("{:?}", cpg.call_graph.calls),
        format!("{:?}", restored.call_graph.calls)
    );
}

#[test]
fn the_metadata_records_the_size_of_the_file_written() {
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    let on_disk = fs::metadata(cache_bin_path(dir.path())).unwrap().len();
    let meta: serde_json::Value =
        serde_json::from_slice(&fs::read(cache_meta_path(dir.path())).unwrap()).unwrap();
    assert_eq!(meta["cache_size_bytes"].as_u64(), Some(on_disk));
    assert_eq!(
        meta["node_count"].as_u64(),
        Some(cpg.graph.node_count() as u64)
    );
    assert_eq!(
        meta["edge_count"].as_u64(),
        Some(cpg.graph.edge_count() as u64)
    );
}

#[test]
fn a_truncated_file_is_a_miss() {
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    let path = cache_bin_path(dir.path());
    let bytes = fs::read(&path).unwrap();
    for keep in [0, 3, 4, 11, bytes.len() / 2, bytes.len() - 1] {
        fs::write(&path, &bytes[..keep]).unwrap();
        assert!(
            matches!(load_cache(&hashes, false, dir.path()), CacheResult::Miss),
            "a file cut to {keep} bytes must be a miss"
        );
    }
}

#[test]
fn a_length_prefix_larger_than_the_file_is_a_miss_without_allocating_it() {
    // Bytes 4..12 are the length of the first string (`prism_version`). A stream decoder with
    // no bound would allocate that many bytes before reading them; 2^62 aborts the process.
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    let path = cache_bin_path(dir.path());
    let mut bytes = fs::read(&path).unwrap();
    for claimed in [1u64 << 62, bytes.len() as u64, (bytes.len() as u64) * 4] {
        bytes[4..12].copy_from_slice(&claimed.to_le_bytes());
        fs::write(&path, &bytes).unwrap();
        assert!(
            matches!(load_cache(&hashes, false, dir.path()), CacheResult::Miss),
            "a claimed length of {claimed} must be a miss"
        );
    }
}

#[test]
fn trailing_bytes_after_a_valid_cache_do_not_make_it_a_miss() {
    // The slice decoder allowed trailing bytes; keep that, so the bound is the only change.
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    let path = cache_bin_path(dir.path());
    let mut bytes = fs::read(&path).unwrap();
    bytes.extend_from_slice(b"trailing");
    fs::write(&path, &bytes).unwrap();
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Hit(_)
    ));
}

#[test]
fn a_write_that_cannot_start_fails_and_leaves_the_previous_cache_usable() {
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    // A directory where the temp file goes makes the create fail.
    fs::create_dir(dir.path().join("cpg-cache.bin.tmp")).unwrap();
    assert!(save_cache(&cpg, &hashes, false, dir.path()).is_err());
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Hit(_)
    ));
}
