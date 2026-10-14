//! Read-side tests for the cache file: the file is mapped and decoded by `read_cache_file`.

use super::super::{cache_bin_path, load_cache, save_cache, CacheResult};
use super::read_cache_file;
use super::tests::{fixture, fixture_of, graph_dump, owned_encoding};
use std::fs;

/// A second, different program (it has call sites, which the fixture requires).
const OTHER_SOURCE: &str = "def a(x):\n    y = b(x)\n    return y\n\ndef b(v):\n    t = v\n    return t\n\ndef c(q):\n    return a(q)\n";

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
    // Characterization of the current loader (unchanged by the streaming writer). Bytes 4..12
    // are the length of the first string (`prism_version`); a claimed length beyond the file
    // must be a Miss, and 2^62 must not be allocated.
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
    // Characterization of the current loader (unchanged by the streaming writer): the slice
    // decoder allows trailing bytes after a complete cache.
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
fn an_empty_cache_file_is_a_miss() {
    let dir = tempfile::tempdir().unwrap();
    let (_cpg, hashes) = fixture();
    let path = cache_bin_path(dir.path());
    fs::write(&path, b"").unwrap();
    assert!(read_cache_file(&path).is_none());
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Miss
    ));
}

#[test]
fn a_directory_at_the_cache_path_is_a_miss() {
    // On unix `File::open` succeeds on a directory and its length is not zero, so the empty
    // check passes and the mapping itself must fail cleanly.
    let dir = tempfile::tempdir().unwrap();
    let (_cpg, hashes) = fixture();
    let path = cache_bin_path(dir.path());
    fs::create_dir(&path).unwrap();
    assert!(read_cache_file(&path).is_none());
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Miss
    ));
}

#[test]
fn an_absent_cache_file_is_a_miss() {
    let dir = tempfile::tempdir().unwrap();
    let (_cpg, hashes) = fixture();
    assert!(read_cache_file(&cache_bin_path(dir.path())).is_none());
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Miss
    ));
}

#[test]
fn a_save_after_a_load_leaves_the_loaded_graph_alone_and_the_next_load_sees_the_new_file() {
    let dir = tempfile::tempdir().unwrap();
    let (cpg_a, hashes_a) = fixture();
    let (cpg_b, hashes_b) = fixture_of(OTHER_SOURCE);
    assert_ne!(graph_dump(&cpg_a), graph_dump(&cpg_b), "graphs must differ");
    save_cache(&cpg_a, &hashes_a, false, dir.path()).unwrap();
    let CacheResult::Hit(first) = load_cache(&hashes_a, false, dir.path()) else {
        panic!("expected a hit for the first graph");
    };
    save_cache(&cpg_b, &hashes_b, false, dir.path()).unwrap();
    assert_eq!(graph_dump(&cpg_a), graph_dump(&first), "loaded graph moved");
    let CacheResult::Hit(second) = load_cache(&hashes_b, false, dir.path()) else {
        panic!("expected a hit for the second graph");
    };
    assert_eq!(graph_dump(&cpg_b), graph_dump(&second));
    assert!(
        !matches!(
            load_cache(&hashes_a, false, dir.path()),
            CacheResult::Hit(_)
        ),
        "the first graph's hashes must no longer hit"
    );
}

#[test]
fn a_save_replaces_the_cache_file_instead_of_rewriting_it() {
    // The mapping's one hazard is the file changing under it. Hold the old file open and
    // map it; a save must leave that file's bytes and the mapping untouched.
    let dir = tempfile::tempdir().unwrap();
    let (cpg_a, hashes_a) = fixture();
    let (cpg_b, hashes_b) = fixture_of(OTHER_SOURCE);
    save_cache(&cpg_a, &hashes_a, false, dir.path()).unwrap();
    let path = cache_bin_path(dir.path());
    let before = fs::read(&path).unwrap();
    let held = fs::File::open(&path).unwrap();
    // SAFETY: read-only; the assertion below is that a save does not modify this file.
    let map = unsafe { memmap2::Mmap::map(&held) }.unwrap();
    save_cache(&cpg_b, &hashes_b, false, dir.path()).unwrap();
    assert_eq!(&map[..], &before[..], "a save changed the mapped file");
    assert_eq!(held.metadata().unwrap().len(), before.len() as u64);
    assert_ne!(
        fs::read(&path).unwrap(),
        before,
        "the name must be replaced"
    );
}

#[test]
fn the_decoded_graph_matches_the_saved_one_node_for_node_and_edge_for_edge() {
    // `a_round_trip_keeps_every_node_and_edge_at_its_index` (stream_tests.rs) already compares
    // every node and edge with its index and endpoints; this one pins it for the mapped read
    // of a freshly saved file, with endpoints that are not symmetric.
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(restored) = load_cache(&hashes, false, dir.path()) else {
        panic!("expected a hit");
    };
    let (nodes, edges) = graph_dump(&restored);
    assert_eq!(nodes.len(), cpg.graph.node_count());
    assert_eq!(edges.len(), cpg.graph.edge_count());
    assert!(cpg.graph.edge_indices().any(|e| {
        let (s, t) = cpg.graph.edge_endpoints(e).unwrap();
        s.index() < t.index()
    }));
    assert_eq!((nodes, edges), graph_dump(&cpg));
}
