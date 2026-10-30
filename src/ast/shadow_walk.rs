//! Shadow checks without restarting tree-sitter's root search for each parent.
use super::ParsedFile;
use tree_sitter::Node;

/// Collect actual ancestors, excluding `node`, in outer-to-inner order.
/// `child_with_descendant` is also the primitive tree-sitter uses for `parent`.
/// Starting each step at the previous child avoids repeated root descents.
/// Byte ranges alone cannot establish ancestry (notably for empty nodes).
fn ancestors<'tree>(mut root: Node<'tree>, node: Node<'tree>) -> Option<Vec<Node<'tree>>> {
    // Cloned/incrementally edited trees can share node IDs but have different
    // ancestry. Only descend in the reference's own tree (no unsafe access).
    if root.into_raw().tree != node.into_raw().tree {
        return None;
    }
    let mut path = Vec::new();
    while root.id() != node.id() {
        path.push(root);
        #[cfg(test)]
        DESCENTS.with(|count| count.set(count.get() + 1));
        root = root.child_with_descendant(node)?;
    }
    Some(path)
}

pub(super) fn is_shadowed_at(
    parsed: &ParsedFile,
    node: Node<'_>,
    func_node: Node<'_>,
    var_name: &str,
    def_line: usize,
) -> bool {
    // The old walk starts at node.parent(): node == func_node must walk ABOVE
    // that node, rather than treating it as an immediately reached boundary.
    let path = (node.id() != func_node.id())
        .then(|| ancestors(func_node, node))
        .flatten()
        .or_else(|| ancestors(parsed.tree.root_node(), node));
    if let Some(path) = path {
        return shadowed_in(
            parsed,
            path.into_iter().rev(),
            func_node,
            var_name,
            def_line,
        );
    }

    // Compatibility for a caller supplying a node from another tree. The file
    // root cannot locate it; retain the original semantics for that rare input.
    // Normal callers, including nodes outside func_node, use the descent above.
    shadowed_in(
        parsed,
        std::iter::successors(node.parent(), |parent| parent.parent()),
        func_node,
        var_name,
        def_line,
    )
}

fn shadowed_in<'tree>(
    parsed: &ParsedFile,
    ancestors: impl Iterator<Item = Node<'tree>>,
    func_node: Node<'_>,
    var_name: &str,
    def_line: usize,
) -> bool {
    for parent in ancestors {
        if parent.id() == func_node.id() {
            break;
        }
        if parsed.language.is_scope_block(parent.kind()) {
            let scope_start = parent.start_position().row + 1;
            let scope_end = parent.end_position().row + 1;
            let def_in_scope = def_line >= scope_start && def_line <= scope_end;
            if !def_in_scope && parsed.scope_has_declaration(parent, var_name) {
                return true;
            }
        }
    }
    false
}

/// Unmodified pre-optimization oracle; deliberately independent of the helpers.
#[cfg(test)]
fn legacy(
    parsed: &ParsedFile,
    node: Node<'_>,
    func_node: Node<'_>,
    var_name: &str,
    def_line: usize,
) -> bool {
    let func_id = func_node.id();
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.id() == func_id {
            break;
        }
        if parsed.language.is_scope_block(parent.kind()) {
            let scope_start = parent.start_position().row + 1;
            let scope_end = parent.end_position().row + 1;
            let def_in_scope = def_line >= scope_start && def_line <= scope_end;
            if !def_in_scope && parsed.scope_has_declaration(parent, var_name) {
                return true;
            }
        }
        current = parent.parent();
    }
    false
}

#[cfg(test)]
thread_local! {
    static DESCENTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
mod tests;
