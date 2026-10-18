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
/// Captured-write admission is outside the const-specialized search walkers.
pub(crate) fn admission_fence(
    p: &ParsedFile,
    node: &Node<'_>,
    owner: &Node<'_>,
    name: &str,
    byte: usize,
) -> bool {
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
    crate::ast::member_reference_index::identifiers::IdentifierReferenceSearch::new(parsed)
        .scoped::<COUNT>(func_node, var_name, def_line)
}
