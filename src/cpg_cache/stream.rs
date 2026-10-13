//! Streaming I/O for the CPG cache file.
//!
//! Writing serializes straight from the live graph into a buffered file, so neither a copy of
//! the graph nor the encoded bytes are held in memory. Reading decodes from a buffered file, so
//! the file's bytes are never held either. The byte format is exactly the one
//! `bincode::serialize(&CpgCache)` produces; `stream_tests.rs` holds the two encodings equal.

use super::{CpgCache, CpgEdge, CpgNode};
use crate::call_graph::CallGraph;
use crate::cpg::{DfgLabelStats, ReturnFlowStats};
use crate::data_flow::DataFlowGraph;

use bincode::Options;
use petgraph::graph::DiGraph;
use serde::ser::{SerializeSeq, Serializer};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::Path;

/// Buffer between the codec and the file, in both directions.
const IO_BUFFER_BYTES: usize = 1 << 20;

/// Borrowed mirror of [`CpgCache`]: the same fields in the same order, so the two encode to
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

/// Encode `cache` into a new file at `path` and return the number of bytes written.
/// The caller removes the file if this fails.
pub(super) fn write_cache_file(path: &Path, cache: &CpgCacheRef<'_>) -> io::Result<u64> {
    let file = File::create(path)?;
    let mut writer = CountingWriter {
        inner: BufWriter::with_capacity(IO_BUFFER_BYTES, file),
        written: 0,
    };
    bincode::serialize_into(&mut writer, cache).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("bincode serialize error: {}", e),
        )
    })?;
    writer.flush()?;
    Ok(writer.written)
}

/// Decode the cache file at `path`. `None` when it is absent, unreadable or not a cache.
///
/// The decoder may consume at most the file's own length. Without that bound a damaged length
/// prefix would make the stream decoder allocate whatever the prefix says before reading it;
/// the slice decoder this replaces refused such a prefix because the bytes were not there.
pub(super) fn read_cache_file(path: &Path) -> Option<CpgCache> {
    let file = File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let reader = BufReader::with_capacity(IO_BUFFER_BYTES, file);
    // `bincode::deserialize`'s configuration (fixed-width integers, trailing bytes allowed),
    // plus the bound.
    bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes()
        .with_limit(len)
        .deserialize_from(reader)
        .ok()
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod tests;
