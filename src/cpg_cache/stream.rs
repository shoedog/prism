//! Streaming writer for the CPG cache file.
//!
//! This module only writes. It serializes straight from the live graph into a buffered file,
//! so neither a copy of the graph nor the encoded bytes are held in memory. The byte format is
//! exactly the one `bincode::serialize(&CpgCache)` produces; `stream_tests.rs` holds the two
//! encodings equal. (Reading still loads the file into memory and decodes the slice; a
//! streamed or mapped reader is a separate change, measured on its own.)

use super::{CpgEdge, CpgNode};
use crate::call_graph::CallGraph;
use crate::cpg::{DfgLabelStats, ReturnFlowStats};
use crate::data_flow::DataFlowGraph;

use petgraph::graph::DiGraph;
use serde::ser::{SerializeSeq, Serializer};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::Path;

#[cfg(test)]
thread_local! {
    /// Test seam: run once on this thread, just before a publish renames its temp file.
    pub(super) static BEFORE_RENAME: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };

    /// Test seam: the result the next save's lock attempt reports instead of asking the file.
    pub(super) static LOCK_RESULT: std::cell::RefCell<Option<fs::TryLockError>> =
        const { std::cell::RefCell::new(None) };
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

/// The one temp name. At most one process writes in a cache directory at a time (see
/// [`publish_file`]), so it never needs to be unique.
const TEMP_NAME: &str = "cpg-cache.bin.tmp";

/// The per-directory writer lock. The file is permanent: it is never truncated, written or
/// removed, so its inode never changes and every writer locks the same file.
const LOCK_NAME: &str = "cpg-cache.lock";

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

/// Write a new file and move it to `final_path`. Returns the number of bytes written, or
/// `None` if another process is saving in `cache_dir` and this save was skipped.
///
/// A save takes an exclusive advisory lock on the permanent `cpg-cache.lock` in `cache_dir`
/// and holds it through the write, the rename and the cleanup after a failure. While it is
/// held no other writer can be using the fixed temp name, so a leftover temp file is an
/// orphan from a crashed writer and `File::create` may truncate it. Nothing ever deletes
/// another writer's file. A writer that finds the lock taken skips its save and reports
/// success: the writer holding it publishes an equivalent cache.
///
/// Only a filesystem that reports that it has no locks ([`lock_is_unsupported`]) is saved to
/// without the lock; concurrent saves there behave as they did before the lock existed (one
/// fixed temp name, write, rename). Any other lock failure skips the save like a taken lock,
/// since it cannot show that nobody else is writing.
///
/// If encoding, flushing or the rename fails, the temp file is removed and `final_path` is
/// left as it was.
pub(super) fn publish_file(
    cache_dir: &Path,
    final_path: &Path,
    encode: impl FnOnce(&mut dyn Write) -> io::Result<()>,
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
        Ok(written)
    });
    if result.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }
    drop(file);
    drop(lock);
    result.map(Some)
}

/// Remove the temp file a crashed writer left behind, if no writer is active (best effort).
///
/// Called on load, so a directory that is read but never saved again does not keep an
/// orphan. The file is deleted only while this call holds the directory lock, which a live
/// writer holds for its whole save. If the lock file or the directory does not exist there is
/// nothing to coordinate and nothing is created or removed; if the lock is taken or cannot
/// be used, the temp file is left alone.
pub(super) fn remove_orphan_temp(cache_dir: &Path) {
    let Ok(lock) = File::options()
        .read(true)
        .write(true)
        .open(cache_dir.join(LOCK_NAME))
    else {
        return;
    };
    if lock.try_lock().is_ok() {
        let _ = fs::remove_file(cache_dir.join(TEMP_NAME));
    }
}

/// Encode `cache` into the temp file in `cache_dir`, then rename it to `final_path`, and
/// return the number of bytes written, or `None` if the save was skipped.
pub(super) fn publish_cache_file(
    cache_dir: &Path,
    final_path: &Path,
    cache: &CpgCacheRef<'_>,
) -> io::Result<Option<u64>> {
    publish_file(cache_dir, final_path, |writer| {
        bincode::serialize_into(writer, cache)
            .map_err(|e| io::Error::other(format!("bincode serialize error: {}", e)))
    })
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod tests;
