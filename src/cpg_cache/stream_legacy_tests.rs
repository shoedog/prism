//! The cache file's names, the choice between a mapped and a buffered read, and the removal of
//! the files that earlier versions of prism left in the directory.

use super::super::{cache_bin_path, load_cache, save_cache, CacheResult};
use super::tests::{fixture, owned_encoding};
use super::{remove_stale_legacy_files, BUFFERED_READS, LEGACY_MIN_AGE, LOCK_RESULT, MAPPED_READS};
use std::fs;
use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime};

const LEGACY: [&str; 3] = ["cpg-cache.bin", "cpg-cache.bin.tmp", "cache-meta.json"];

/// Set a file's modification time to `age` before now (or after, with `later`).
fn set_age(path: &Path, age: Duration, later: bool) {
    let now = SystemTime::now();
    let when = if later { now + age } else { now - age };
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(when)
        .unwrap();
}

fn hours(count: u64) -> Duration {
    Duration::from_secs(count * 60 * 60)
}

fn plant_legacy(dir: &Path, age: Duration) {
    for name in LEGACY {
        fs::write(dir.join(name), b"legacy").unwrap();
        set_age(&dir.join(name), age, false);
    }
}

/// A directory with a saved cache (so the lock file exists) and the hashes that hit it.
fn saved() -> (
    tempfile::TempDir,
    std::collections::BTreeMap<String, String>,
) {
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    (dir, hashes)
}

fn hold_lock(dir: &Path) -> fs::File {
    let file = fs::File::options()
        .read(true)
        .write(true)
        .open(dir.join("cpg-cache.lock"))
        .unwrap();
    file.try_lock().unwrap();
    file
}

fn reset_counters() {
    MAPPED_READS.with(|c| c.set(0));
    BUFFERED_READS.with(|c| c.set(0));
}

fn reads() -> (u32, u32) {
    (
        MAPPED_READS.with(|c| c.get()),
        BUFFERED_READS.with(|c| c.get()),
    )
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

#[test]
fn the_reader_never_reads_the_legacy_cache_name() {
    let dir = tempfile::tempdir().unwrap();
    let (cpg, hashes) = fixture();
    fs::write(
        dir.path().join("cpg-cache.bin"),
        owned_encoding(&cpg, &hashes),
    )
    .unwrap();
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Miss
    ));
}

#[test]
fn a_save_writes_only_the_v2_names_and_the_lock() {
    let (dir, _hashes) = saved();
    assert_eq!(
        names(dir.path()),
        ["cache-meta-v2.json", "cpg-cache-v2.bin", "cpg-cache.lock"]
    );
}

#[test]
fn a_save_leaves_the_legacy_files_alone() {
    let dir = tempfile::tempdir().unwrap();
    plant_legacy(dir.path(), hours(48));
    let (cpg, hashes) = fixture();
    save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    for name in LEGACY {
        assert_eq!(
            fs::read(dir.path().join(name)).unwrap(),
            b"legacy",
            "{name}"
        );
    }
}

#[test]
fn a_load_removes_legacy_files_idle_for_more_than_a_day_and_keeps_the_cache() {
    let (dir, hashes) = saved();
    plant_legacy(dir.path(), LEGACY_MIN_AGE + hours(1));
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Hit(_)
    ));
    for name in LEGACY {
        assert!(!dir.path().join(name).exists(), "{name} should be removed");
    }
    assert!(cache_bin_path(dir.path()).exists());
    assert!(dir.path().join("cache-meta-v2.json").exists());
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Hit(_)
    ));
}

#[test]
fn a_load_keeps_legacy_files_that_are_not_yet_a_day_idle() {
    let (dir, hashes) = saved();
    plant_legacy(dir.path(), LEGACY_MIN_AGE - hours(1));
    let _ = load_cache(&hashes, false, dir.path());
    for name in LEGACY {
        assert!(
            dir.path().join(name).exists(),
            "{name} is only 23 hours old"
        );
    }
}

#[test]
fn a_legacy_file_dated_in_the_future_is_kept() {
    let (dir, hashes) = saved();
    plant_legacy(dir.path(), hours(0));
    for name in LEGACY {
        set_age(&dir.path().join(name), hours(48), true);
    }
    let _ = load_cache(&hashes, false, dir.path());
    for name in LEGACY {
        assert!(dir.path().join(name).exists(), "{name}");
    }
}

#[test]
fn a_load_keeps_old_legacy_files_while_another_process_holds_the_lock() {
    let (dir, hashes) = saved();
    plant_legacy(dir.path(), hours(48));
    let other = hold_lock(dir.path());
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Hit(_)
    ));
    for name in LEGACY {
        assert!(dir.path().join(name).exists(), "{name}");
    }
    drop(other);
    let _ = load_cache(&hashes, false, dir.path());
    for name in LEGACY {
        assert!(
            !dir.path().join(name).exists(),
            "{name} once the lock is free"
        );
    }
}

#[test]
fn a_load_without_a_lock_file_removes_no_legacy_file() {
    let dir = tempfile::tempdir().unwrap();
    plant_legacy(dir.path(), hours(48));
    let (_cpg, hashes) = fixture();
    let _ = load_cache(&hashes, false, dir.path());
    for name in LEGACY {
        assert!(dir.path().join(name).exists(), "{name}");
    }
    assert!(!dir.path().join("cpg-cache.lock").exists());
}

#[cfg(unix)]
#[test]
fn a_symlink_under_a_legacy_name_is_neither_followed_nor_removed() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("target");
    fs::write(&target, b"precious").unwrap();
    set_age(&target, hours(48), false);
    let link = dir.path().join("cpg-cache.bin");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let link_to_dir = dir.path().join("cache-meta.json");
    std::os::unix::fs::symlink(outside.path(), &link_to_dir).unwrap();
    // A regular file under the third name, so the call is seen to remove something.
    let plain = dir.path().join("cpg-cache.bin.tmp");
    fs::write(&plain, b"legacy").unwrap();

    // First with the real clock: the target is old but the links themselves are new.
    remove_stale_legacy_files(dir.path(), SystemTime::now());
    assert!(
        fs::symlink_metadata(&link).is_ok(),
        "followed the link's target's age"
    );
    assert!(plain.exists());
    // Then with a clock two days ahead, so the links' own modification times are old too.
    remove_stale_legacy_files(dir.path(), SystemTime::now() + hours(48));
    assert!(!plain.exists(), "the regular file is idle by now");
    assert!(fs::symlink_metadata(&link).is_ok(), "a symlink was removed");
    assert!(
        fs::symlink_metadata(&link_to_dir).is_ok(),
        "a symlink was removed"
    );
    assert_eq!(fs::read(&target).unwrap(), b"precious");
    assert!(outside.path().is_dir());
}

#[test]
fn a_load_maps_the_file_when_the_lock_works() {
    let (dir, hashes) = saved();
    reset_counters();
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Hit(_)
    ));
    assert_eq!(reads(), (1, 0), "(mapped, buffered)");
}

#[test]
fn a_load_maps_the_file_while_another_process_holds_the_lock() {
    let (dir, hashes) = saved();
    let other = hold_lock(dir.path());
    reset_counters();
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Hit(_)
    ));
    assert_eq!(reads(), (1, 0), "a held lock is a working lock");
    drop(other);
}

#[test]
fn a_load_buffers_the_file_when_the_lock_is_unsupported_or_fails_and_still_loads() {
    for error in [
        io::Error::from(io::ErrorKind::Unsupported),
        io::Error::other("lock failed"),
        io::Error::from(io::ErrorKind::PermissionDenied),
    ] {
        let (dir, hashes) = saved();
        reset_counters();
        let description = error.to_string();
        LOCK_RESULT.with(|slot| *slot.borrow_mut() = Some(fs::TryLockError::Error(error)));
        let CacheResult::Hit(_) = load_cache(&hashes, false, dir.path()) else {
            panic!("a lock error must not turn a valid cache into a miss: {description}");
        };
        assert_eq!(reads(), (0, 1), "(mapped, buffered) after: {description}");
    }
}

#[test]
fn a_load_buffers_the_file_when_there_is_no_lock_file_and_still_loads() {
    let (dir, hashes) = saved();
    fs::remove_file(dir.path().join("cpg-cache.lock")).unwrap();
    reset_counters();
    assert!(matches!(
        load_cache(&hashes, false, dir.path()),
        CacheResult::Hit(_)
    ));
    assert_eq!(reads(), (0, 1), "(mapped, buffered)");
    assert!(
        !dir.path().join("cpg-cache.lock").exists(),
        "a load creates no lock file"
    );
}
