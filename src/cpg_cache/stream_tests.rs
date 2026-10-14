use super::super::{
    cache_bin_path, cache_meta_path, compute_file_hashes, compute_topology_key,
    current_cache_build_identity, load_cache, save_cache, CacheResult, CpgCache, SerializedCpg,
    CACHE_VERSION, SKIP_POLICY_VERSION,
};
use crate::cpg::{CodePropertyGraph, CpgEdge, CpgNode};
use std::collections::BTreeMap;
use std::fs;

const SOURCE: &str = "class C:\n    def m(self, p):\n        q = p\n        return q\n\ndef g(c):\n    r = c.m(1)\n    return r\n\ndef h(x):\n    return g(x)\n";

pub(super) fn fixture() -> (CodePropertyGraph, BTreeMap<String, String>) {
    fixture_of(SOURCE)
}

/// The fixture graph and file hashes for one Python source (`t.py`).
pub(super) fn fixture_of(source: &str) -> (CodePropertyGraph, BTreeMap<String, String>) {
    let mut sources = BTreeMap::new();
    sources.insert("t.py".to_string(), source.to_string());
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
pub(super) fn owned_encoding(
    cpg: &CodePropertyGraph,
    hashes: &BTreeMap<String, String>,
) -> Vec<u8> {
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

pub(super) fn graph_dump(cpg: &CodePropertyGraph) -> (Vec<String>, Vec<String>) {
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
fn a_write_that_fails_part_way_leaves_no_temp_file_and_the_previous_cache_intact() {
    let dir = tempfile::tempdir().unwrap();
    let final_path = dir.path().join("cpg-cache.bin");
    fs::write(&final_path, b"previous").unwrap();
    let after_called = std::cell::Cell::new(false);
    let result = super::publish_file(
        dir.path(),
        &final_path,
        |writer| {
            writer.write_all(b"partial")?;
            Err(std::io::Error::other("disk full"))
        },
        |_| after_called.set(true),
    );
    assert!(!after_called.get(), "nothing was published to follow up");
    assert_eq!(result.unwrap_err().to_string(), "disk full");
    assert_eq!(temp_files(dir.path()), Vec::<String>::new());
    assert_eq!(fs::read(&final_path).unwrap(), b"previous");

    // The lock was released with the failure: the next write goes through.
    let written = super::publish_file(
        dir.path(),
        &final_path,
        |writer| writer.write_all(b"whole"),
        |_| {},
    );
    assert_eq!(written.unwrap(), Some(5));
    assert_eq!(fs::read(&final_path).unwrap(), b"whole");
    assert_eq!(temp_files(dir.path()), Vec::<String>::new());
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

const LOCK_NAME: &str = "cpg-cache.lock";

fn hold_lock(dir: &std::path::Path) -> fs::File {
    let file = fs::File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join(LOCK_NAME))
        .unwrap();
    file.try_lock().unwrap();
    file
}

#[test]
fn a_save_skips_when_another_writer_holds_the_directory_lock() {
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    let planted = b"the other writer's published cache".to_vec();
    fs::write(cache_bin_path(dir.path()), &planted).unwrap();

    let other = hold_lock(dir.path());
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();

    assert!(fs::read(cache_bin_path(dir.path())).unwrap() == planted);
    assert!(!dir.path().join("cpg-cache.bin.tmp").exists());
    assert_eq!(temp_files(dir.path()), Vec::<String>::new());
    drop(other);
}

#[test]
fn a_save_after_the_other_writer_lets_go_writes_the_cache() {
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    fs::write(cache_bin_path(dir.path()), b"stale").unwrap();

    let other = hold_lock(dir.path());
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    assert!(fs::read(cache_bin_path(dir.path())).unwrap() == b"stale");
    drop(other);

    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Hit(_)
    ));
}

#[test]
fn an_orphan_temp_file_of_any_age_is_overwritten_by_the_next_save() {
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    let orphan = dir.path().join("cpg-cache.bin.tmp");
    fs::write(&orphan, vec![0xAB; 4096]).unwrap();

    save_cache(&cpg, &hashes, false, dir.path()).unwrap();

    assert!(!orphan.exists(), "the orphan is renamed away as the cache");
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Hit(_)
    ));
}

#[cfg(unix)]
#[test]
fn the_lock_file_is_permanent_and_keeps_its_inode() {
    use std::os::unix::fs::MetadataExt;
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    let lock = dir.path().join(LOCK_NAME);

    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    let first = fs::metadata(&lock).unwrap().ino();
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    let second = fs::metadata(&lock).unwrap().ino();

    assert_eq!(first, second);
    assert_eq!(
        fs::metadata(&lock).unwrap().len(),
        0,
        "the lock is never written"
    );
}

#[test]
fn the_lock_is_held_through_the_rename() {
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    let lock_path = dir.path().join(LOCK_NAME);
    let seen = std::rc::Rc::new(std::cell::Cell::new(None));
    let seen_in_hook = seen.clone();
    super::BEFORE_RENAME.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move || {
            let other = fs::File::options()
                .read(true)
                .write(true)
                .open(&lock_path)
                .unwrap();
            seen_in_hook.set(Some(matches!(
                other.try_lock(),
                Err(std::fs::TryLockError::WouldBlock)
            )));
        }));
    });

    save_cache(&cpg, &hashes, false, dir.path()).unwrap();

    assert_eq!(seen.get(), Some(true), "the hook ran with the lock held");
}

#[test]
fn a_load_removes_an_orphan_temp_file_when_no_writer_holds_the_lock() {
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Hit(_)
    ));
    let orphan = dir.path().join("cpg-cache.bin.tmp");
    fs::write(&orphan, vec![0xCD; 4096]).unwrap();

    // The orphan changes nothing about what the load returns.
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Hit(_)
    ));
    assert!(!orphan.exists(), "the orphan outlived a load");
    let mut changed = hashes.clone();
    changed.insert("other.py".to_string(), "0".repeat(64));
    assert!(!matches!(
        load_cache(&changed, false, dir.path()),
        CacheResult::Hit(_)
    ));
    assert!(dir.path().join(LOCK_NAME).exists(), "the lock file stays");
}

#[test]
fn a_load_removes_an_orphan_temp_file_even_when_there_is_no_cache() {
    let dir = tempfile::tempdir().unwrap();
    let orphan = dir.path().join("cpg-cache.bin.tmp");
    drop(hold_lock(dir.path()));
    fs::write(&orphan, b"x").unwrap();
    assert!(matches!(
        load_cache(&BTreeMap::new(), false, dir.path()),
        CacheResult::Miss
    ));
    assert!(!orphan.exists());
}

#[test]
fn a_load_leaves_the_temp_file_of_a_live_writer() {
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    let orphan = dir.path().join("cpg-cache.bin.tmp");
    fs::write(&orphan, b"half-written").unwrap();

    let writer = hold_lock(dir.path());
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Hit(_)
    ));
    assert_eq!(fs::read(&orphan).unwrap(), b"half-written");
    drop(writer);
}

#[test]
fn a_load_without_a_lock_file_leaves_the_temp_file_and_creates_no_lock_file() {
    let dir = tempfile::tempdir().unwrap();
    let orphan = dir.path().join("cpg-cache.bin.tmp");
    fs::write(&orphan, b"x").unwrap();
    let _ = load_cache(&BTreeMap::new(), false, dir.path());
    assert!(orphan.exists());
    assert!(!dir.path().join(LOCK_NAME).exists());

    // A directory that does not exist is not created either.
    let missing = dir.path().join("no-such-dir");
    let _ = load_cache(&BTreeMap::new(), false, &missing);
    assert!(!missing.exists());
}

fn inject_lock_error(error: std::io::Error) {
    super::LOCK_RESULT.with(|slot| *slot.borrow_mut() = Some(std::fs::TryLockError::Error(error)));
}

#[test]
fn an_operational_lock_error_skips_the_save_and_touches_no_temp_file() {
    // The lock is really held by a writer mid-save, but this save's attempt reports an error
    // that does not mean "locking is unsupported" (lock table exhausted, no memory, ...).
    for kind in [
        std::io::ErrorKind::OutOfMemory,
        std::io::ErrorKind::Other,
        std::io::ErrorKind::PermissionDenied,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (cpg, hashes) = fixture();
        let previous = b"previous cache".to_vec();
        fs::write(cache_bin_path(dir.path()), &previous).unwrap();
        let live_temp = dir.path().join("cpg-cache.bin.tmp");
        fs::write(&live_temp, b"another writer, half written").unwrap();
        let writer = hold_lock(dir.path());

        inject_lock_error(std::io::Error::from(kind));
        save_cache(&cpg, &hashes, false, dir.path()).unwrap();

        assert!(
            fs::read(&live_temp).unwrap() == b"another writer, half written",
            "{kind:?}: the holder's temp file was touched"
        );
        assert!(
            fs::read(cache_bin_path(dir.path())).unwrap() == previous,
            "{kind:?}: the previous cache changed"
        );
        drop(writer);
    }
}

#[test]
fn a_lock_error_that_means_locking_is_unsupported_saves_without_the_lock() {
    let mut errors = vec![std::io::Error::from(std::io::ErrorKind::Unsupported)];
    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
    errors.push(std::io::Error::from_raw_os_error(45)); // ENOTSUP, which std leaves uncategorized
    for error in errors {
        let dir = tempfile::tempdir().unwrap();
        let (cpg, hashes) = fixture();
        let description = format!("{error:?}");
        inject_lock_error(error);
        save_cache(&cpg, &hashes, false, dir.path()).unwrap();
        assert!(
            matches!(load_cache(&hashes, false, dir.path()), CacheResult::Hit(_)),
            "{description}: the unlocked save did not publish"
        );
        assert_eq!(temp_files(dir.path()), Vec::<String>::new());
    }
}

#[cfg(unix)]
#[test]
fn a_lock_file_this_user_cannot_write_still_serializes_and_saves() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    let lock = dir.path().join(LOCK_NAME);
    fs::write(&lock, b"").unwrap();
    fs::set_permissions(&lock, fs::Permissions::from_mode(0o444)).unwrap();

    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Hit(_)
    ));

    // And it still excludes: a holder of the same read-only file makes the next save skip.
    let holder = fs::File::open(&lock).unwrap();
    holder.try_lock().unwrap();
    let published = fs::read(cache_bin_path(dir.path())).unwrap();
    fs::remove_file(cache_bin_path(dir.path())).unwrap();
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    assert!(
        !cache_bin_path(dir.path()).exists(),
        "{} bytes",
        published.len()
    );
    drop(holder);
}

#[test]
fn the_follow_up_to_a_publish_runs_under_the_lock_and_gets_the_size() {
    // The metadata is written by the locked saver, so it describes the file just published.
    let dir = tempfile::tempdir().unwrap();
    let final_path = dir.path().join("cpg-cache.bin");
    let lock_path = dir.path().join(LOCK_NAME);
    let seen = std::cell::Cell::new(None);
    let written = super::publish_file(
        dir.path(),
        &final_path,
        |writer| writer.write_all(b"whole"),
        |size| {
            let other = fs::File::options()
                .read(true)
                .write(true)
                .open(&lock_path)
                .unwrap();
            let blocked = matches!(other.try_lock(), Err(std::fs::TryLockError::WouldBlock));
            seen.set(Some((size, blocked, final_path.exists())));
        },
    );
    assert_eq!(written.unwrap(), Some(5));
    assert_eq!(seen.get(), Some((5, true, true)));
}

#[test]
fn a_skipped_publish_runs_no_follow_up() {
    let dir = tempfile::tempdir().unwrap();
    let final_path = dir.path().join("cpg-cache.bin");
    let holder = hold_lock(dir.path());
    let called = std::cell::Cell::new(false);
    let written = super::publish_file(
        dir.path(),
        &final_path,
        |writer| writer.write_all(b"whole"),
        |_| called.set(true),
    );
    assert_eq!(written.unwrap(), None);
    assert!(!called.get());
    assert!(!final_path.exists());
    drop(holder);
}

#[test]
fn a_save_reports_whether_it_wrote_and_a_skip_writes_no_metadata() {
    use super::super::SaveOutcome;
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();

    let holder = hold_lock(dir.path());
    let skipped = save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    assert_eq!(skipped, SaveOutcome::Skipped);
    assert!(!cache_meta_path(dir.path()).exists());
    drop(holder);

    let saved = save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    assert_eq!(saved, SaveOutcome::Saved);
    assert!(cache_meta_path(dir.path()).exists());
}
