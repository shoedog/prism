//! The reads of one JS/TS/TSX owner, found by line (#660 S1b-2).
//!
//! D3's use endpoint is usually a zero-width line anchor. Its read occurrences are found the way
//! S1a places a use in a region (`capture_regions::resolve_key`: full path first, base name when no
//! read on the line spells it, every occurrence kept), over the same read collector. A use with real
//! bytes is placed by them.
use super::capture_regions::{candidate_path, resolve_key, Candidate, Collector};
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use tree_sitter::Node;

pub(super) struct ReadIndex {
    /// Every read of the owner, sorted by line.
    reads: Vec<Candidate>,
}

impl ReadIndex {
    pub(super) fn build(owner: Node<'_>) -> Self {
        let mut reads = Vec::new();
        Collector::new(&mut reads).walk_children(owner);
        reads.sort_by_key(|c| c.0);
        Self { reads }
    }

    /// The byte ranges of the reads on `line` that spell `path`.
    pub(super) fn on_line(
        &self,
        parsed: &ParsedFile,
        path: &AccessPath,
        line: usize,
    ) -> Vec<(usize, usize)> {
        let start = self.reads.partition_point(|c| c.0 < line);
        let on_line: Vec<&Candidate> = self.reads[start..]
            .iter()
            .take_while(|c| c.0 == line)
            .collect();
        let spelled = |wanted: &AccessPath| {
            let found: Vec<(usize, usize)> = on_line
                .iter()
                .filter(|c| candidate_path(parsed, c) == *wanted)
                .map(|c| (c.1, c.2))
                .collect();
            (!found.is_empty()).then_some(found)
        };
        resolve_key(path, spelled).unwrap_or_default()
    }
}

/// The read node with exactly these bytes.
pub(super) fn read_node<'t>(parsed: &'t ParsedFile, start: usize, end: usize) -> Option<Node<'t>> {
    let node = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(start, end)?;
    (node.start_byte() == start && node.end_byte() == end).then_some(node)
}
