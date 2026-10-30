//! Streaming writer for the CPG cache file.
//!
//! Writing serializes straight from the live graph into a buffered file, so neither a copy of
//! the graph nor the encoded bytes are held in memory. The byte format is exactly the one
//! `bincode::serialize(&CpgCache)` produces; `stream_tests.rs` holds the two encodings equal.
//! Reading maps the file and decodes the mapped slice, so the file's bytes are never copied
//! into the heap either (`read_cache_file`).

use super::{CpgCache, CpgEdge, CpgNode};
use crate::call_graph::CallGraph;
use crate::cpg::{DfgLabelStats, ReturnFlowStats};
use crate::data_flow::DataFlowGraph;

use petgraph::graph::DiGraph;
use serde::ser::{SerializeSeq, Serializer};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, BufWriter, Read, Write};
use std::path::Path;
use std::time::{Duration, SystemTime};

#[cfg(test)]
thread_local! {
    /// Test seam: run once on this thread, just before a publish renames its temp file.
    pub(super) static BEFORE_RENAME: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };

    /// Test seam: the result the next save's lock attempt reports instead of asking the file.
    pub(super) static LOCK_RESULT: std::cell::RefCell<Option<fs::TryLockError>> =
        const { std::cell::RefCell::new(None) };

    /// Test seam: how many reads on this thread went through a mapping, and how many through a
    /// buffer, so a test can see which way `read_cache_file` chose.
    pub(super) static MAPPED_READS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    pub(super) static BUFFERED_READS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// `File::try_lock`, except that a test can make the next call report a chosen result.
fn try_lock_file(file: &File) -> Result<(), fs::TryLockError> {
    #[cfg(test)]
    if let Some(injected) = LOCK_RESULT.with(|slot| slot.borrow_mut().take()) {
        return Err(injected);
    }
    file.try_lock()
}

/// Buffer between the codec and the file.
const IO_BUFFER_BYTES: usize = 1 << 20;

/// The published cache file. The `-v2` names are ones that no earlier prism ever touches: the
/// file is only ever replaced by a rename from [`TEMP_NAME`] and never written in place, and
/// no older binary holds an open descriptor to its inode (see [`read_cache_file`]).
pub(super) const CACHE_FILE_NAME: &str = "cpg-cache-v2.bin";

/// The metadata that describes [`CACHE_FILE_NAME`].
pub(super) const META_FILE_NAME: &str = "cache-meta-v2.json";

/// The one temp name. At most one process writes in a cache directory at a time (see
/// [`publish_file`]), so it never needs to be unique.
const TEMP_NAME: &str = "cpg-cache-v2.bin.tmp";

/// The per-directory writer lock. The file is permanent: it is never truncated, written or
/// removed, so its inode never changes and every writer locks the same file. Earlier versions
/// of prism that took this lock use the same name, which is why it did not change.
const LOCK_NAME: &str = "cpg-cache.lock";

/// The files earlier versions of prism wrote in a cache directory. Nothing reads or writes them
/// any more; a load removes each one that has been idle for [`LEGACY_MIN_AGE`].
const LEGACY_NAMES: [&str; 3] = ["cpg-cache.bin", "cpg-cache.bin.tmp", "cache-meta.json"];

/// How long a legacy file must have gone unmodified before a load removes it. An older binary
/// that saved within the last day keeps its files. One that has only read them for longer (a
/// hit does not rewrite or touch the file) may find them removed; its next load then misses and
/// rebuilds once, and the descriptor or mapping it already holds is not disturbed, so the
/// result is still correct. That is the accepted migration cost until older binaries are
/// replaced. A temp file a crashed legacy writer left behind is reclaimed a day later.
pub(super) const LEGACY_MIN_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// Borrowed mirror of `CpgCache`: the same fields in the same order, so the two encode to
/// the same bytes.
#[derive(Serialize)]
pub(super) struct CpgCacheRef<'a> {
    pub(super) version: u32,
    pub(super) prism_version: &'a str,
    pub(super) grammar_fingerprint: &'a str,
    pub(super) cache_build_identity: &'a str,
    pub(super) git_sha: &'a str,
    pub(super) skip_policy_version: u32,
    pub(super) file_hashes: &'a BTreeMap<String, String>,
    pub(super) topology_key: &'a BTreeMap<String, String>,
    pub(super) has_type_db: bool,
    pub(super) graph: SerializedCpgRef<'a>,
}

/// Borrowed mirror of `SerializedCpg`.
#[derive(Serialize)]
pub(super) struct SerializedCpgRef<'a> {
    pub(super) nodes: NodeSeq<'a>,
    pub(super) edges: EdgeSeq<'a>,
    pub(super) call_graph: &'a CallGraph,
    pub(super) dfg: &'a DataFlowGraph,
    pub(super) return_flow_stats: &'a ReturnFlowStats,
    pub(super) dfg_label_stats: &'a DfgLabelStats,
}

/// The graph's nodes in index order, encoded as `Vec<CpgNode>` is.
pub(super) struct NodeSeq<'a>(pub(super) &'a DiGraph<CpgNode, CpgEdge>);

impl Serialize for NodeSeq<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.0.node_count()))?;
        for index in self.0.node_indices() {
            seq.serialize_element(&self.0[index])?;
        }
        seq.end()
    }
}

/// The graph's edges in index order, encoded as `Vec<(u32, u32, CpgEdge)>` is.
pub(super) struct EdgeSeq<'a>(pub(super) &'a DiGraph<CpgNode, CpgEdge>);

impl Serialize for EdgeSeq<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.0.edge_count()))?;
        for index in self.0.edge_indices() {
            let (source, target) = self
                .0
                .edge_endpoints(index)
                .expect("an index from edge_indices has endpoints");
            seq.serialize_element(&(source.index() as u32, target.index() as u32, &self.0[index]))?;
        }
        seq.end()
    }
}

struct CountingWriter<W> {
    inner: W,
    written: u64,
}

impl<W: Write> Write for CountingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let count = self.inner.write(buf)?;
        self.written += count as u64;
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Let `encode` write `file` through a buffer and return the number of bytes written. The
/// buffer is flushed before returning, so a flush error is reported here and not lost.
pub(super) fn write_new_file(
    file: &File,
    encode: impl FnOnce(&mut dyn Write) -> io::Result<()>,
) -> io::Result<u64> {
    let mut writer = CountingWriter {
        inner: BufWriter::with_capacity(IO_BUFFER_BYTES, file),
        written: 0,
    };
    encode(&mut writer).and_then(|()| writer.flush())?;
    Ok(writer.written)
}

/// Open the permanent lock file, creating it if needed. A file this user can see but not
/// write (a directory shared between users, a lock file created with a stricter umask) is
/// opened read-only instead: a local advisory lock does not depend on the open mode.
fn open_lock_file(cache_dir: &Path) -> io::Result<File> {
    let path = cache_dir.join(LOCK_NAME);
    let opened = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path);
    match opened {
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => File::open(&path).or(Err(e)),
        other => other,
    }
}

/// Open the lock file only if it already exists, creating nothing.
fn open_existing_lock_file(cache_dir: &Path) -> Option<File> {
    File::options()
        .read(true)
        .write(true)
        .open(cache_dir.join(LOCK_NAME))
        .ok()
}

/// Whether a failed lock attempt means this filesystem has no advisory locks at all (as
/// opposed to a transient or operational failure on one that does).
///
/// `ErrorKind::Unsupported` covers `ENOSYS` and `EOPNOTSUPP`; std leaves `ENOTSUP`
/// uncategorized where it differs from `EOPNOTSUPP` (macOS and the BSDs), so that one is
/// compared by value. `libc` is an optional dependency, so the numbers are written here.
fn lock_is_unsupported(error: &io::Error) -> bool {
    if error.kind() == io::ErrorKind::Unsupported {
        return true;
    }
    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
    const ENOTSUP: i32 = 45;
    #[cfg(any(target_os = "netbsd", target_os = "openbsd"))]
    const ENOTSUP: i32 = 91;
    #[cfg(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    if error.raw_os_error() == Some(ENOTSUP) {
        return true;
    }
    false
}

/// Write a new file and move it to `final_path`, then run `after_publish` with the number of
/// bytes written. Returns that number, or `None` if the save was skipped.
///
/// A save takes an exclusive advisory lock on the permanent `cpg-cache.lock` in `cache_dir`
/// and holds it through the write, the rename, `after_publish` (the metadata write) and the
/// cleanup after a failure. While it is held no other writer can be using the fixed temp
/// name, so a leftover temp file is an orphan from a crashed writer and `File::create` may
/// truncate it. Nothing deletes another writer's file: a load removes the orphan only while
/// it holds the same lock ([`clean_after_load`]).
///
/// A save that does not get the lock is skipped and reported as `None`: no file is written
/// and `after_publish` does not run. The holder is another saver, or a load (checking that the
/// lock works, or cleaning up after itself); a saver that later fails, or one with different
/// inputs, leaves the cache absent or stale. A skip is a lost best-effort save, not a promise
/// that an equivalent cache will appear: every load revalidates the file hashes against the sources.
///
/// Only a filesystem that reports that it has no locks ([`lock_is_unsupported`]) is saved to
/// without the lock; concurrent saves there behave as they did before the lock existed (one
/// fixed temp name, write, rename). Any other lock failure skips the save like a taken lock,
/// since it cannot show that nobody else is writing.
///
/// The guarantee assumes one coherent lock across every process that shares the directory.
/// It does not hold on a network filesystem mounted with local-only locking (NFS `nolock` or
/// `local_lock`), and a binary from before the lock existed neither takes nor honors it.
///
/// If encoding, flushing or the rename fails, the temp file is removed and `final_path` is
/// left as it was.
pub(super) fn publish_file(
    cache_dir: &Path,
    final_path: &Path,
    encode: impl FnOnce(&mut dyn Write) -> io::Result<()>,
    after_publish: impl FnOnce(u64),
) -> io::Result<Option<u64>> {
    let lock = open_lock_file(cache_dir)?;
    match try_lock_file(&lock) {
        Ok(()) => {}
        Err(fs::TryLockError::Error(e)) if lock_is_unsupported(&e) => {}
        // Taken by another process, or a failure that says nothing about whether the
        // directory is in use: either way the temp file is not ours to touch.
        Err(_) => return Ok(None),
    }
    let tmp_path = cache_dir.join(TEMP_NAME);
    let file = File::create(&tmp_path)?;
    let result = write_new_file(&file, encode).and_then(|written| {
        #[cfg(test)]
        BEFORE_RENAME.with(|hook| {
            if let Some(hook) = hook.borrow_mut().take() {
                hook();
            }
        });
        fs::rename(&tmp_path, final_path)?;
        after_publish(written);
        Ok(written)
    });
    if result.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }
    drop(file);
    drop(lock);
    result.map(Some)
}

/// Housekeeping after a load, done only while this call holds the directory lock (best effort).
///
/// It removes the temp file a crashed writer left behind, so a directory that is read but never
/// saved again does not keep an orphan, and the files of earlier prism versions that have been
/// idle for [`LEGACY_MIN_AGE`] ([`remove_stale_legacy_files`]). A live writer holds the lock for
/// its whole save, so neither is touched during a save. If the lock file or the directory does
/// not exist there is nothing to coordinate and nothing is created or removed; if the lock is
/// taken or cannot be used, everything is left alone.
pub(super) fn clean_after_load(cache_dir: &Path) {
    let Some(lock) = open_existing_lock_file(cache_dir) else {
        return;
    };
    if lock.try_lock().is_ok() {
        let _ = fs::remove_file(cache_dir.join(TEMP_NAME));
        remove_stale_legacy_files(cache_dir, SystemTime::now());
    }
}

/// Remove each legacy cache file in `cache_dir` whose modification time is more than
/// [`LEGACY_MIN_AGE`] before `now`. Only regular files are removed: a symlink or a directory
/// under a legacy name is left alone, and a symlink is never followed. A file with a
/// modification time in the future, or none, is kept. Errors are ignored.
///
/// Another process could replace a legacy file between the check and the removal; the loss is
/// one stale file that an older prism would rebuild.
pub(super) fn remove_stale_legacy_files(cache_dir: &Path, now: SystemTime) {
    for name in LEGACY_NAMES {
        let path = cache_dir.join(name);
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if !meta.file_type().is_file() {
            continue;
        }
        let idle_for = meta
            .modified()
            .ok()
            .and_then(|modified| now.duration_since(modified).ok());
        if idle_for.is_some_and(|idle| idle > LEGACY_MIN_AGE) {
            let _ = fs::remove_file(&path);
        }
    }
}

/// Whether the directory lock works on this filesystem: the lock file exists and a `try_lock`
/// on it either succeeds (released at once) or reports that someone holds it. A missing lock
/// file, an unsupported lock ([`lock_is_unsupported`]) or any other failure means no.
///
/// A map of the cache file is safe only if every writer of this directory is serialized by the
/// lock (see [`read_cache_file`]); without working locks they are not.
fn directory_lock_works(cache_dir: &Path) -> bool {
    let Some(lock) = open_existing_lock_file(cache_dir) else {
        return false;
    };
    match try_lock_file(&lock) {
        Ok(()) => {
            let _ = lock.unlock();
            true
        }
        Err(fs::TryLockError::WouldBlock) => true,
        // `lock_is_unsupported` errors and every other error are the same answer here.
        Err(fs::TryLockError::Error(_)) => false,
    }
}

/// Encode `cache` into the temp file in `cache_dir`, then rename it to `final_path`, and
/// return the number of bytes written, or `None` if the save was skipped. See
/// [`publish_file`] for `after_publish`.
pub(super) fn publish_cache_file(
    cache_dir: &Path,
    final_path: &Path,
    cache: &CpgCacheRef<'_>,
    after_publish: impl FnOnce(u64),
) -> io::Result<Option<u64>> {
    publish_file(
        cache_dir,
        final_path,
        |writer| {
            bincode::serialize_into(writer, cache)
                .map_err(|e| io::Error::other(format!("bincode serialize error: {}", e)))
        },
        after_publish,
    )
}

/// Decode the cache file at `path`. `None` when it is absent, unreadable or not a cache.
///
/// When the directory lock works ([`directory_lock_works`]) the file is mapped read-only rather
/// than read into a buffer: its pages are the page cache's, not this process's heap, so a load no
/// longer holds a heap copy of the whole file next to the decoded index, and decoding the mapped
/// slice keeps the speed and the bounds checks of the slice decoder that `fs::read` +
/// `bincode::deserialize` used. The mapping is dropped before this function returns. Without a
/// working lock the file is read into a buffer, as before.
///
/// A mapped read on a network filesystem shared between hosts, or on a disk that reports an I/O
/// error, can fault (SIGBUS) where a buffered read returned an error and a miss.
pub(super) fn read_cache_file(path: &Path) -> Option<CpgCache> {
    let mut file = File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    if len == 0 {
        // An empty file is not a cache. memmap2 0.9.8 maps one as an empty slice, which also
        // fails to decode; this keeps that a miss without depending on the crate's behavior.
        return None;
    }
    if !path.parent().is_some_and(directory_lock_works) {
        #[cfg(test)]
        BUFFERED_READS.with(|count| count.set(count.get() + 1));
        let mut data = Vec::new();
        file.read_to_end(&mut data).ok()?;
        return decode_current_cache(&data);
    }
    #[cfg(test)]
    MAPPED_READS.with(|count| count.set(count.get() + 1));
    // SAFETY: the mapping is read-only and lives only for this function, and the mapped bytes
    // never change while it exists. The file can change under a mapping only by being written
    // or truncated through some descriptor after it got this name. Here, only a current writer
    // writes `cpg-cache-v2.bin.tmp`, and it does so under the directory lock and then renames
    // it to `cpg-cache-v2.bin`; it ends its writes before the rename and never opens the
    // published name for writing, so a published inode is not modified again. Older versions of
    // prism use other names (`cpg-cache.bin`, `cpg-cache.bin.tmp`) and never touch these.
    // That relies on the lock serializing current writers, which is why this branch is taken
    // only when the lock works: without locks two current writers share the fixed temp name
    // and one can still be writing an inode the other has just published, so the file is read
    // into a buffer instead. Like `publish_file`, this assumes one coherent lock across every
    // process that shares the directory: not NFS mounted with `nolock` or `local_lock` across
    // hosts, and not a directory shared between a host and a VM. Tests that rewrite the file in
    // place do it in private directories.
    let map = unsafe { memmap2::Mmap::map(&file) }.ok()?;
    decode_current_cache(&map[..])
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "stream_read_tests.rs"]
mod read_tests;

#[cfg(test)]
#[path = "stream_legacy_tests.rs"]
mod legacy_tests;

/// Check the fixed-width prefix before the incompatible payload is decoded.
fn decode_current_cache(data: &[u8]) -> Option<CpgCache> {
    let version: u32 = bincode::deserialize(data.get(..4)?).ok()?;
    if version != super::CACHE_VERSION {
        eprintln!(
            "Cache version mismatch (cached: {}, current: {}), rebuilding",
            version,
            super::CACHE_VERSION
        );
        return None;
    }
    #[cfg(test)]
    PAYLOAD_DECODES.with(|n| n.set(n.get() + 1));
    bincode::deserialize(data).ok()
}
#[cfg(test)]
thread_local! { pub(super) static PAYLOAD_DECODES: std::cell::Cell<u32> = const { std::cell::Cell::new(0) }; }
#[cfg(test)]
#[path = "activation_tests.rs"]
pub(super) mod activation_tests;
