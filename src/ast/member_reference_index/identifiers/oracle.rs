//! Frozen pre-index query searches; test-only differential oracle.
use crate::ast::*;

pub(crate) fn legacy_scoped_refs<const COUNT: bool>(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    var_name: &str,
    def_line: usize,
) -> std::collections::BTreeSet<usize> {
    use crate::profile::gates::shadow;
    use crate::queries::{get_query, QueryKind};
    use std::collections::BTreeSet;
    use tree_sitter::StreamingIterator;

    if let Some(query) = get_query(parsed.language, QueryKind::Identifiers) {
        #[cfg(test)]
        crate::ast::member_reference_index::identifiers::record_query();
        let ident_idx = query
            .capture_index_for_name("ident")
            .expect("Identifiers query must have @ident capture");
        let mut cursor = tree_sitter::QueryCursor::new();
        cursor.set_byte_range(func_node.byte_range());
        let mut matches = cursor.matches(query, parsed.tree.root_node(), parsed.source.as_bytes());
        let mut lines = BTreeSet::new();
        while let Some(m) = matches.next() {
            for capture in m.captures {
                super::record_capture();
                if capture.index == ident_idx && parsed.node_text(&capture.node) == var_name {
                    // Bottom-up scope check: walk from the capture node up to the
                    // function boundary. If any enclosing scope block re-declares
                    // var_name and doesn't contain def_line, skip this reference.
                    if !shadow::<COUNT>(parsed, &capture.node, func_node, var_name, def_line) {
                        lines.insert(capture.node.start_position().row + 1);
                    }
                }
            }
        }
        return lines;
    }

    let mut lines = BTreeSet::new();
    parsed.collect_variable_refs_scoped_manual(*func_node, var_name, def_line, &mut lines);
    lines
}

impl ParsedFile {
    #[allow(clippy::too_many_arguments, clippy::nonminimal_bool)]
    pub(crate) fn legacy_collect_fenced_simple_refs<const COUNT: bool>(
        &self,
        node: Node<'_>,
        owner: &Node<'_>,
        name: &str,
        def_line: usize,
        def_byte: usize,
        reads_only: bool,
        out: &mut BTreeMap<usize, (usize, usize)>,
    ) {
        use crate::profile::gates::{fence, shadow};
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;
        let Some(query) = get_query(self.language, QueryKind::Identifiers) else {
            out.extend(
                legacy_scoped_refs::<COUNT>(self, owner, name, def_line)
                    .into_iter()
                    .map(|line| {
                        (
                            line,
                            (self.line_start_byte(line), self.line_start_byte(line)),
                        )
                    }),
            );
            return;
        };
        let ident_idx = query
            .capture_index_for_name("ident")
            .expect("Identifiers query must have @ident capture");
        #[cfg(test)]
        super::record_query();
        let mut cursor = tree_sitter::QueryCursor::new();
        cursor.set_byte_range(node.byte_range());
        let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
        while let Some(m) = matches.next() {
            for capture in m.captures {
                super::record_capture();
                if capture.index == ident_idx
                    && self.node_text(&capture.node) == name
                    && !shadow::<COUNT>(self, &capture.node, owner, name, def_line)
                    && !fence::<COUNT>(self, &capture.node, owner, name, def_byte)
                    && !(reads_only && self.js_ts_identifier_is_not_a_read(capture.node))
                    && !(reads_only && self.js_ts_reference_in_with_body(capture.node))
                {
                    out.entry(capture.node.start_position().row + 1)
                        .or_insert((capture.node.start_byte(), capture.node.end_byte()));
                }
            }
        }
    }
}
