//! Const-specialized gates: COUNT=false compiles to the original call only.
use crate::ast::ParsedFile;
use tree_sitter::Node;

fn count(update: impl FnOnce(&mut super::owner::Counters)) {
    super::owner::COUNTERS.with(|slot| {
        if let Some(c) = slot.borrow_mut().as_mut() {
            update(c);
        }
    });
}
pub(crate) fn fence<const COUNT: bool>(
    p: &ParsedFile,
    node: &Node<'_>,
    owner: &Node<'_>,
    name: &str,
    byte: usize,
) -> bool {
    if COUNT {
        count(|c| c.binding_fence_calls += 1);
    }
    p.js_ts_reference_fenced(node, owner, name, byte)
}
pub(crate) fn shadow<const COUNT: bool>(
    p: &ParsedFile,
    node: &Node<'_>,
    owner: &Node<'_>,
    name: &str,
    line: usize,
) -> bool {
    if COUNT {
        count(|c| c.shadow_check_calls += 1);
    }
    p.is_shadowed_at(node, owner, name, line)
}
pub(crate) fn def_scope(
    p: &ParsedFile,
    owner: &Node<'_>,
    name: &str,
    byte: usize,
) -> Option<(usize, usize)> {
    if super::enabled() {
        count(|c| c.def_scope_calls += 1);
    }
    p.js_ts_def_scope(owner, name, byte)
}

pub(crate) fn scoped_refs<const COUNT: bool>(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    var_name: &str,
    def_line: usize,
) -> std::collections::BTreeSet<usize> {
    use crate::queries::{get_query, QueryKind};
    use std::collections::BTreeSet;
    use tree_sitter::StreamingIterator;

    if let Some(query) = get_query(parsed.language, QueryKind::Identifiers) {
        let ident_idx = query
            .capture_index_for_name("ident")
            .expect("Identifiers query must have @ident capture");
        let mut cursor = tree_sitter::QueryCursor::new();
        cursor.set_byte_range(func_node.byte_range());
        let mut matches = cursor.matches(query, parsed.tree.root_node(), parsed.source.as_bytes());
        let mut lines = BTreeSet::new();
        while let Some(m) = matches.next() {
            for capture in m.captures {
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
