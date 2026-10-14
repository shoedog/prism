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
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

#[cfg(test)]
thread_local! {
    /// Test seam: run once on this thread, just before a publish renames its temp file.
    pub(super) static BEFORE_RENAME: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

/// Buffer between the codec and the file.
const IO_BUFFER_BYTES: usize = 1 << 20;

/// A temp file nobody has written to for this long belongs to a writer that died.
const STALE_TEMP_AGE: Duration = Duration::from_secs(15 * 60);

const TEMP_PREFIX: &str = "cpg-cache.bin.";
const TEMP_SUFFIX: &str = ".tmp";
/// The one fixed temp name used before temp names were unique.
const LEGACY_TEMP_NAME: &str = "cpg-cache.bin.tmp";

/// How many temp names are tried before an `AlreadyExists` is given up on.
const CREATE_ATTEMPTS: usize = 16;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A temp file name no other writer in this process uses: this process's id and a counter.
///
/// The name is unique among live writers of one pid namespace. Writers that share a cache
/// directory across namespaces, or a reused pid, can still land on the same name, so
/// [`create_temp_file`] creates it exclusively and moves on to the next name if it is taken.
pub(super) fn unique_temp_path(cache_dir: &Path) -> PathBuf {
    let nonce = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    cache_dir.join(format!("{TEMP_PREFIX}{pid}.{nonce}{TEMP_SUFFIX}"))
}

/// Whether `name` is a temp file name this module (or the version before it) generates:
/// `cpg-cache.bin.<digits>.<digits>.tmp` or the legacy `cpg-cache.bin.tmp`. Nothing else in the
/// cache directory is ours to delete.
fn is_temp_name(name: &str) -> bool {
    if name == LEGACY_TEMP_NAME {
        return true;
    }
    let Some(middle) = name
        .strip_prefix(TEMP_PREFIX)
        .and_then(|rest| rest.strip_suffix(TEMP_SUFFIX))
    else {
        return false;
    };
    let all_digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    middle
        .split_once('.')
        .is_some_and(|(pid, nonce)| all_digits(pid) && all_digits(nonce))
}

/// Create a temp file nobody else has, lock it, and return it with its path.
///
/// Creation is exclusive (`O_EXCL`): a name that is already taken is left untouched and the
/// next one from `next_path` is tried, up to [`CREATE_ATTEMPTS`] times. The exclusive advisory
/// lock marks the writer as alive; it lasts as long as the returned `File` is open, so the
/// stale-file sweep leaves the file alone for the whole save. A filesystem that cannot lock
/// leaves the file unlocked, and the sweep then falls back to the file's age alone.
pub(super) fn create_temp_file(
    mut next_path: impl FnMut() -> PathBuf,
) -> io::Result<(PathBuf, File)> {
    let mut attempt = 0;
    loop {
        let path = next_path();
        match File::options().write(true).create_new(true).open(&path) {
            Ok(file) => {
                return match file.try_lock() {
                    Ok(()) | Err(fs::TryLockError::Error(_)) => Ok((path, file)),
                    Err(fs::TryLockError::WouldBlock) => {
                        // A file this call just created, locked by someone else: not ours.
                        drop(file);
                        let _ = fs::remove_file(&path);
                        Err(io::Error::from(io::ErrorKind::WouldBlock))
                    }
                };
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists && attempt + 1 < CREATE_ATTEMPTS => {
                attempt += 1;
            }
            Err(e) => return Err(e),
        }
    }
}

/// Remove temp files left by writers that died mid-write (best effort). A unique name is
/// never overwritten by the next writer, so without this each crash would leave a partial
/// cache behind for good.
///
/// A file is removed only if it matches a temp name exactly, has not been modified for
/// [`STALE_TEMP_AGE`], and no writer holds its lock: a writer that was suspended (or whose
/// clock jumped) keeps its lock, so age alone never condemns a live writer. A file whose lock
/// the filesystem does not support is judged by age alone.
pub(super) fn remove_stale_temp_files(cache_dir: &Path, now: SystemTime) {
    let Ok(entries) = fs::read_dir(cache_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !is_temp_name(name) {
            continue;
        }
        let modified = entry.metadata().and_then(|meta| meta.modified());
        let stale = match modified {
            Ok(modified) => now
                .duration_since(modified)
                .is_ok_and(|age| age >= STALE_TEMP_AGE),
            Err(_) => false,
        };
        if !stale {
            continue;
        }
        let Ok(file) = File::open(entry.path()) else {
            continue;
        };
        // Held until the file is gone, so no writer can take the lock in between.
        match file.try_lock() {
            Ok(()) | Err(fs::TryLockError::Error(_)) => {
                let _ = fs::remove_file(entry.path());
            }
            Err(fs::TryLockError::WouldBlock) => {}
        }
    }
}

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

/// Write a new file and move it to `final_path`, returning the number of bytes written.
///
/// The bytes go to a freshly created, locked temp file (see [`create_temp_file`]) that is
/// renamed over `final_path` only once complete; the lock is held until after the rename. If
/// encoding, flushing or the rename fails, the temp file is removed and `final_path` is left
/// as it was.
pub(super) fn publish_file(
    next_path: impl FnMut() -> PathBuf,
    final_path: &Path,
    encode: impl FnOnce(&mut dyn Write) -> io::Result<()>,
) -> io::Result<u64> {
    let (tmp_path, file) = create_temp_file(next_path)?;
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
    result
}

/// Encode `cache` into a new temp file in `cache_dir`, then rename it to `final_path`, and
/// return the number of bytes written.
pub(super) fn publish_cache_file(
    cache_dir: &Path,
    final_path: &Path,
    cache: &CpgCacheRef<'_>,
) -> io::Result<u64> {
    publish_file(
        || unique_temp_path(cache_dir),
        final_path,
        |writer| {
            bincode::serialize_into(writer, cache)
                .map_err(|e| io::Error::other(format!("bincode serialize error: {}", e)))
        },
    )
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod tests;
