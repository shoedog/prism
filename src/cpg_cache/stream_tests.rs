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

fn temp_files(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".tmp"))
        .collect();
    names.sort();
    names
}

#[test]
fn a_graph_with_no_nodes_or_edges_streams_to_the_owned_encoding_and_loads_back() {
    // An explicit sequence length of 0 must encode as an empty vector does.
    let dir = tempfile::tempdir().unwrap();
    let files: BTreeMap<String, crate::ast::ParsedFile> = BTreeMap::new();
    let cpg = CodePropertyGraph::build(&files);
    assert_eq!((cpg.graph.node_count(), cpg.graph.edge_count()), (0, 0));
    let hashes = BTreeMap::new();
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    assert!(fs::read(cache_bin_path(dir.path())).unwrap() == owned_encoding(&cpg, &hashes));
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Hit(_)
    ));
}

#[test]
fn a_save_leaves_no_temp_file_behind() {
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    assert_eq!(temp_files(dir.path()), Vec::<String>::new());
}

#[test]
fn every_writer_gets_its_own_temp_name() {
    // One shared name would let a second writer truncate the file under the first.
    let dir = tempfile::tempdir().unwrap();
    let first = super::unique_temp_path(dir.path());
    let second = super::unique_temp_path(dir.path());
    assert_ne!(first, second);
    for path in [&first, &second] {
        let name = path.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with("cpg-cache.bin."), "{name}");
        assert!(name.ends_with(".tmp"), "{name}");
        assert!(name.contains(&std::process::id().to_string()), "{name}");
        assert_eq!(path.parent(), Some(dir.path()));
    }
}

fn names_from(paths: Vec<std::path::PathBuf>) -> impl FnMut() -> std::path::PathBuf {
    let mut paths = paths.into_iter();
    move || {
        paths
            .next()
            .expect("more temp names requested than planned")
    }
}

#[test]
fn a_write_that_fails_part_way_leaves_nothing_at_its_path() {
    let dir = tempfile::tempdir().unwrap();
    let tmp = dir.path().join("cpg-cache.bin.1.0.tmp");
    let final_path = dir.path().join("cpg-cache.bin");
    let result = super::publish_file(names_from(vec![tmp.clone()]), &final_path, |writer| {
        writer.write_all(b"partial")?;
        Err(std::io::Error::other("disk full"))
    });
    assert_eq!(result.unwrap_err().to_string(), "disk full");
    assert!(!tmp.exists(), "a failed write must remove its file");
    assert!(!final_path.exists());

    let written = super::publish_file(names_from(vec![tmp.clone()]), &final_path, |writer| {
        writer.write_all(b"whole")
    })
    .unwrap();
    assert_eq!(written, 5);
    assert_eq!(fs::read(&final_path).unwrap(), b"whole");
    assert!(!tmp.exists(), "the temp file is renamed away");
}

#[test]
fn a_temp_name_that_is_taken_is_left_intact_and_the_next_name_is_used() {
    // A second writer with the same pid (another container, a reused pid) can pick a name a
    // live writer already created. Creating it must not truncate the other writer's file.
    let dir = tempfile::tempdir().unwrap();
    let taken = dir.path().join("cpg-cache.bin.7.0.tmp");
    let free = dir.path().join("cpg-cache.bin.7.1.tmp");
    let final_path = dir.path().join("cpg-cache.bin");
    fs::write(&taken, b"another writer's half-written file").unwrap();

    let written = super::publish_file(
        names_from(vec![taken.clone(), free.clone()]),
        &final_path,
        |writer| writer.write_all(b"mine"),
    )
    .unwrap();

    assert_eq!(written, 4);
    assert_eq!(
        fs::read(&taken).unwrap(),
        b"another writer's half-written file"
    );
    assert_eq!(fs::read(&final_path).unwrap(), b"mine");
    assert!(!free.exists());
}

#[test]
fn creating_a_temp_file_gives_up_after_a_bounded_number_of_taken_names() {
    let dir = tempfile::tempdir().unwrap();
    let taken = dir.path().join("cpg-cache.bin.7.0.tmp");
    fs::write(&taken, b"x").unwrap();
    let mut asked = 0;
    let error = super::create_temp_file(|| {
        asked += 1;
        taken.clone()
    })
    .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(asked, 16);
    assert_eq!(fs::read(&taken).unwrap(), b"x");
}

#[test]
fn a_rename_that_fails_removes_the_temp_file_and_reports_the_error() {
    // `cpg-cache.bin` is a non-empty directory, so the final rename cannot succeed.
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    let blocker = cache_bin_path(dir.path());
    fs::create_dir(&blocker).unwrap();
    fs::write(blocker.join("occupant"), b"x").unwrap();

    assert!(save_cache(&cpg, &hashes, false, dir.path()).is_err());
    assert_eq!(temp_files(dir.path()), Vec::<String>::new());
    assert!(blocker.join("occupant").exists());
}

#[test]
fn an_old_unlocked_temp_file_is_removed_and_a_recent_one_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    let now = std::time::SystemTime::now();
    let age = |seconds: u64| now - std::time::Duration::from_secs(seconds);
    let plant = |name: &str, modified: std::time::SystemTime| {
        let path = dir.path().join(name);
        fs::write(&path, b"partial").unwrap();
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(modified)
            .unwrap();
    };
    plant("cpg-cache.bin.4242.0.tmp", age(15 * 60)); // exactly at the limit: stale
    plant("cpg-cache.bin.tmp", age(3600)); // the name used before this change
    plant("cpg-cache.bin.4243.7.tmp", age(15 * 60 - 5)); // written to recently: not stale
    plant("resolved-call-edge-index.bin.1.0.tmp", age(3600)); // another cache's temp: not ours
    plant("cpg-cache.bin", age(3600)); // the cache itself is never a temp file

    super::remove_stale_temp_files(dir.path(), now);
    assert_eq!(
        temp_files(dir.path()),
        vec![
            "cpg-cache.bin.4243.7.tmp".to_string(),
            "resolved-call-edge-index.bin.1.0.tmp".to_string()
        ]
    );
    assert!(dir.path().join("cpg-cache.bin").exists());

    // A save runs the same cleanup and still leaves a recently written file alone. The file is
    // stamped "now" right before the save, so no wall-clock margin is involved.
    plant("cpg-cache.bin.4243.7.tmp", std::time::SystemTime::now());
    plant("cpg-cache.bin.4244.0.tmp", age(7200));
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    assert_eq!(
        temp_files(dir.path()),
        vec![
            "cpg-cache.bin.4243.7.tmp".to_string(),
            "resolved-call-edge-index.bin.1.0.tmp".to_string()
        ]
    );
}

#[cfg(unix)]
#[test]
fn a_write_that_cannot_start_fails_and_leaves_the_previous_cache_usable() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    // A directory that cannot be written to makes the temp file's creation fail.
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o555)).unwrap();
    let result = save_cache(&cpg, &hashes, false, dir.path());
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(result.is_err());
    assert_eq!(temp_files(dir.path()), Vec::<String>::new());
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Hit(_)
    ));
}

fn plant_old(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    fs::write(&path, b"partial").unwrap();
    fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(7200))
        .unwrap();
    path
}

#[test]
fn an_old_temp_file_whose_writer_holds_its_lock_is_kept_by_a_save() {
    // A writer suspended for longer than the age limit (or hit by a clock jump) still holds
    // its lock, so a save's sweep must not unlink the file under it.
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    let path = plant_old(dir.path(), "cpg-cache.bin.4245.0.tmp");
    let writer = fs::File::open(&path).unwrap();
    writer.try_lock().unwrap();

    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    assert_eq!(
        temp_files(dir.path()),
        vec!["cpg-cache.bin.4245.0.tmp".to_string()]
    );

    drop(writer);
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    assert_eq!(temp_files(dir.path()), Vec::<String>::new());
}

#[test]
fn a_writer_holds_its_lock_on_the_temp_file_until_the_rename() {
    let dir = tempfile::tempdir().unwrap();
    let tmp = dir.path().join("cpg-cache.bin.9.0.tmp");
    let final_path = dir.path().join("cpg-cache.bin");
    let far_future = std::time::SystemTime::now() + std::time::Duration::from_secs(24 * 3600);
    let seen_locked = std::cell::Cell::new(false);

    super::publish_file(names_from(vec![tmp.clone()]), &final_path, |writer| {
        writer.write_all(b"in progress")?;
        // Mid-write: another process could not lock the file, and its sweep leaves it alone
        // even when the file looks a day old.
        let other = fs::File::open(&tmp)?;
        seen_locked.set(matches!(
            other.try_lock(),
            Err(std::fs::TryLockError::WouldBlock)
        ));
        super::remove_stale_temp_files(dir.path(), far_future);
        assert!(tmp.exists(), "the sweep removed a live writer's file");
        Ok(())
    })
    .unwrap();

    assert!(seen_locked.get(), "the temp file was not locked mid-write");
    assert_eq!(fs::read(&final_path).unwrap(), b"in progress");
}

#[test]
fn only_the_generated_temp_names_are_swept() {
    let dir = tempfile::tempdir().unwrap();
    let now = std::time::SystemTime::now() + std::time::Duration::from_secs(3600);
    let keep = [
        "cpg-cache.bin.notes.tmp",
        "cpg-cache.bin.1.x.tmp",
        "cpg-cache.bin.1.0.extra.tmp",
        "cpg-cache.bin.1.tmp",
        "cpg-cache.bin..0.tmp",
        "cpg-cache.bin.1..tmp",
        "cpg-cache.bin.1.0.tmp.bak",
    ];
    let remove = ["cpg-cache.bin.12.345.tmp", "cpg-cache.bin.tmp"];
    for name in keep.iter().chain(remove.iter()) {
        plant_old(dir.path(), name);
    }
    super::remove_stale_temp_files(dir.path(), now);
    let mut left: Vec<String> = fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    left.sort();
    let mut expected: Vec<String> = keep.iter().map(|name| name.to_string()).collect();
    expected.sort();
    assert_eq!(left, expected);
}
