//! One lazy identifier-query index per immutable parsed-file search session.
//!
//! The pinned queries match single nodes (including nested Terraform captures).
//! Keep the matches/captures stream verbatim: neither sorting nor deduplication
//! is valid. Gates run anew for each search, in that same order.
use crate::ast::ParsedFile;
use crate::profile::gates::{fence, shadow};
use crate::queries::{get_query, QueryKind};
use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ops::Range;
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator};

struct Candidate<'a> {
    node: Node<'a>,
    bytes: Range<usize>,
}

struct Index<'a> {
    by_name: HashMap<&'a str, Vec<Candidate<'a>>>,
}

impl<'a> Index<'a> {
    fn build(parsed: &'a ParsedFile, query: &Query) -> Self {
        #[cfg(test)]
        record_query();
        let ident = query
            .capture_index_for_name("ident")
            .expect("Identifiers query must have @ident capture");
        let mut index = Self {
            by_name: HashMap::new(),
        };
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(query, parsed.tree.root_node(), parsed.source.as_bytes());
        while let Some(m) = matches.next() {
            for capture in m.captures {
                #[cfg(test)]
                record_capture();
                if capture.index == ident {
                    index
                        .by_name
                        .entry(parsed.node_text(&capture.node))
                        .or_default()
                        .push(Candidate {
                            node: capture.node,
                            bytes: capture.node.byte_range(),
                        });
                }
            }
        }
        index
    }

    fn matching<'b>(
        &'b self,
        name: &str,
        bytes: Range<usize>,
    ) -> impl Iterator<Item = Node<'a>> + 'b {
        // QueryCursor casts to u32, treats end=0 as unbounded, and ignores an
        // invalid start>end range, leaving its initial unbounded range intact.
        let mut start = bytes.start as u32 as usize;
        let mut end = bytes.end as u32 as usize;
        if end == 0 {
            end = u32::MAX as usize;
        }
        if start > end {
            start = 0;
            end = u32::MAX as usize;
        }
        self.by_name
            .get(name)
            .into_iter()
            .flatten()
            .filter(move |c| {
                #[cfg(test)]
                EXAMINED.with(|n| n.set(n.get() + 1));
                // set_byte_range admits partial overlaps, not just contained nodes.
                // Empty nodes at start are admitted only if their parent overlaps.
                c.bytes.start < end
                    && (c.bytes.end > start || c.bytes.is_empty() && c.bytes.end == start)
                    && (!c.bytes.is_empty()
                        || c.node
                            .parent()
                            .is_none_or(|p| p.end_byte() > start && p.start_byte() < end))
            })
            .map(|c| c.node)
    }
}

pub(crate) struct IdentifierReferenceSearch<'a> {
    parsed: &'a ParsedFile,
    index: OnceCell<Option<Index<'a>>>,
}

impl<'a> IdentifierReferenceSearch<'a> {
    pub(crate) fn new(parsed: &'a ParsedFile) -> Self {
        Self {
            parsed,
            index: OnceCell::new(),
        }
    }

    fn index(&self) -> Option<&Index<'a>> {
        self.index
            .get_or_init(|| {
                get_query(self.parsed.language, QueryKind::Identifiers)
                    .map(|query| Index::build(self.parsed, query))
            })
            .as_ref()
    }

    pub(crate) fn scoped<const COUNT: bool>(
        &self,
        owner: &Node<'_>,
        name: &str,
        def_line: usize,
    ) -> BTreeSet<usize> {
        let mut lines = BTreeSet::new();
        if let Some(index) = self.index() {
            for node in index.matching(name, owner.byte_range()) {
                if !shadow::<COUNT>(self.parsed, &node, owner, name, def_line) {
                    lines.insert(node.start_position().row + 1);
                }
            }
        } else {
            self.parsed
                .collect_variable_refs_scoped_manual(*owner, name, def_line, &mut lines);
        }
        lines
    }

    #[allow(clippy::too_many_arguments, clippy::nonminimal_bool)]
    pub(super) fn collect_fenced<const COUNT: bool>(
        &self,
        root: Node<'_>,
        owner: &Node<'_>,
        name: &str,
        def_line: usize,
        def_byte: usize,
        reads_only: bool,
        out: &mut BTreeMap<usize, (usize, usize)>,
    ) {
        let parsed = self.parsed;
        let Some(index) = self.index() else {
            // The old no-query fallback uses owner, not the restricted root.
            out.extend(
                self.scoped::<COUNT>(owner, name, def_line)
                    .into_iter()
                    .map(|line| {
                        (
                            line,
                            (parsed.line_start_byte(line), parsed.line_start_byte(line)),
                        )
                    }),
            );
            return;
        };
        for node in index.matching(name, root.byte_range()) {
            if !shadow::<COUNT>(parsed, &node, owner, name, def_line)
                && !fence::<COUNT>(parsed, &node, owner, name, def_byte)
                && !(reads_only && parsed.js_ts_identifier_is_not_a_read(node))
                && !(reads_only && parsed.js_ts_reference_in_with_body(node))
            {
                out.entry(node.start_position().row + 1)
                    .or_insert((node.start_byte(), node.end_byte()));
            }
        }
    }
}

#[cfg(test)]
thread_local! {
    static QUERIES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static CAPTURES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static EXAMINED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
pub(crate) fn record_query() {
    QUERIES.with(|n| n.set(n.get() + 1));
}
#[cfg(test)]
fn record_capture() {
    CAPTURES.with(|n| n.set(n.get() + 1));
}
#[cfg(test)]
pub(crate) mod oracle;
#[cfg(test)]
mod tests;
