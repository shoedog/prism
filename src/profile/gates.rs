//! Const-specialized gates: COUNT=false compiles to the original call only.
use crate::ast::ParsedFile;
use tree_sitter::Node;

#[cfg(test)]
thread_local! {
    pub(crate) static SCOPED_QUERY_WORK: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };
}

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
    #[cfg(test)]
    crate::ast::binding_verdict_memo::probe::gate("fence", node.id(), || {
        format!("{}:{name}:{byte}", owner.id())
    });
    if COUNT {
        count(|c| c.binding_fence_calls += 1);
    }
    p.js_ts_reference_fenced(node, owner, name, byte)
}
/// Captured-write admission is outside the const-specialized search walkers.
pub(crate) fn admission_fence(
    p: &ParsedFile,
    node: &Node<'_>,
    owner: &Node<'_>,
    name: &str,
    byte: usize,
) -> bool {
    #[cfg(test)]
    crate::ast::binding_verdict_memo::probe::gate("fence", node.id(), || {
        format!("{}:{name}:{byte}", owner.id())
    });
    if super::enabled() {
        count(|c| c.binding_fence_calls += 1); // admission increment
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
    #[cfg(test)]
    crate::ast::binding_verdict_memo::probe::gate("shadow", node.id(), || {
        format!("{}:{name}:{line}", owner.id())
    });
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
    #[cfg(test)]
    crate::ast::binding_verdict_memo::probe::gate("def_scope", owner.id(), || {
        format!("{name}:{byte}")
    });
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
        #[cfg(test)]
        SCOPED_QUERY_WORK.with(|c| c.set((c.get().0 + 1, c.get().1)));
        let ident_idx = query
            .capture_index_for_name("ident")
            .expect("Identifiers query must have @ident capture");
        let mut cursor = tree_sitter::QueryCursor::new();
        cursor.set_byte_range(func_node.byte_range());
        let mut matches = cursor.matches(query, parsed.tree.root_node(), parsed.source.as_bytes());
        let mut lines = BTreeSet::new();
        while let Some(m) = matches.next() {
            for capture in m.captures {
                #[cfg(test)]
                SCOPED_QUERY_WORK.with(|c| c.set((c.get().0, c.get().1 + 1)));
                if capture.index == ident_idx
                    && parsed.node_text(&capture.node) == var_name
                    && !parsed.python_import_syntax(capture.node)
                {
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
