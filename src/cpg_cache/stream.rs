//! Streaming I/O for the CPG cache file.
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
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

/// Buffer between the codec and the file, in both directions.
const IO_BUFFER_BYTES: usize = 1 << 20;

/// A temp file nobody has written to for this long belongs to a writer that died.
const STALE_TEMP_AGE: Duration = Duration::from_secs(15 * 60);

const TEMP_PREFIX: &str = "cpg-cache.bin.";
const TEMP_SUFFIX: &str = ".tmp";

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A temp file name no other writer uses: this process's id and a counter.
///
/// The name must be unique because a streamed write takes as long as the encoding does. With
/// one shared name, a second writer's `create` would truncate the file under the first, and
/// the first could then rename a file with a hole in it into place.
pub(super) fn unique_temp_path(cache_dir: &Path) -> PathBuf {
    let nonce = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    cache_dir.join(format!("{TEMP_PREFIX}{pid}.{nonce}{TEMP_SUFFIX}"))
}

/// Remove temp files left by writers that died mid-write (best effort). A unique name is
/// never overwritten by the next writer, so without this each crash would leave a partial
/// cache behind for good. A live writer touches its file continuously and is left alone.
pub(super) fn remove_stale_temp_files(cache_dir: &Path, now: SystemTime) {
    let Ok(entries) = fs::read_dir(cache_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !(name.starts_with(TEMP_PREFIX) && name.ends_with(TEMP_SUFFIX)) {
            continue;
        }
        let modified = entry.metadata().and_then(|meta| meta.modified());
        let stale = match modified {
            Ok(modified) => now
                .duration_since(modified)
                .is_ok_and(|age| age >= STALE_TEMP_AGE),
            Err(_) => false,
        };
        if stale {
            let _ = fs::remove_file(entry.path());
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

/// Create `path`, let `encode` write it through a buffer, and return the number of bytes
/// written. If creating, encoding or flushing fails, nothing is left at `path`.
pub(super) fn write_new_file(
    path: &Path,
    encode: impl FnOnce(&mut dyn Write) -> io::Result<()>,
) -> io::Result<u64> {
    let file = File::create(path)?;
    let mut writer = CountingWriter {
        inner: BufWriter::with_capacity(IO_BUFFER_BYTES, file),
        written: 0,
    };
    match encode(&mut writer).and_then(|()| writer.flush()) {
        Ok(()) => Ok(writer.written),
        Err(e) => {
            drop(writer);
            let _ = fs::remove_file(path);
            Err(e)
        }
    }
}

/// Encode `cache` into a new file at `path` and return the number of bytes written.
pub(super) fn write_cache_file(path: &Path, cache: &CpgCacheRef<'_>) -> io::Result<u64> {
    write_new_file(path, |writer| {
        bincode::serialize_into(writer, cache)
            .map_err(|e| io::Error::other(format!("bincode serialize error: {}", e)))
    })
}

/// Decode the cache file at `path`. `None` when it is absent, unreadable or not a cache.
///
/// The file is mapped read-only rather than read into a buffer: its pages are the page cache's,
/// not this process's heap, so a load no longer holds a heap copy of the whole file next to the
/// decoded index, and decoding the mapped slice keeps the speed and the bounds checks of the
/// slice decoder that `fs::read` + `bincode::deserialize` used. The mapping is dropped before this
/// function returns.
pub(super) fn read_cache_file(path: &Path) -> Option<CpgCache> {
    let file = File::open(path).ok()?;
    if file.metadata().ok()?.len() == 0 {
        // Mapping an empty file is an error on some platforms; an empty file is not a cache.
        return None;
    }
    // SAFETY: the mapping is read-only and lives only for this function. The one hazard of a
    // file mapping is the file shrinking while it is mapped, which faults the process. Every
    // writer of `cpg-cache.bin` (this module, and every earlier version of prism) writes a
    // separate temp file and renames it over the name, which replaces the directory entry and
    // leaves the mapped file untouched; nothing truncates or rewrites it in place.
    let map = unsafe { memmap2::Mmap::map(&file) }.ok()?;
    bincode::deserialize(&map[..]).ok()
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod tests;
