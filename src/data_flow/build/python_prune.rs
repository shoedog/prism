//! Python facts for rules U and D (#630 slice 5). Removal-only, lexical.
use crate::ast::ParsedFile;
use std::collections::BTreeSet;
use tree_sitter::Node;

/// Scopes the producer does not split from the owner: a def or use inside one is
/// never proven (comprehensions are scopes; a class body binds its own names).
pub(super) const NESTED_SCOPES: [&str; 5] = [
    "list_comprehension",
    "dictionary_comprehension",
    "set_comprehension",
    "generator_expression",
    "class_definition",
];

/// Single-entry region: control enters a `block` only at its first statement.
pub(super) fn is_region(kind: &str) -> bool {
    kind == "block"
}

/// Names declared `global`/`nonlocal` anywhere in the owner (nested defs included):
/// another callable may write them between a dominating write and a read.
pub(super) fn note_scope_declaration(
    parsed: &ParsedFile,
    node: Node<'_>,
    out: &mut BTreeSet<String>,
) {
    if matches!(node.kind(), "global_statement" | "nonlocal_statement") {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            out.insert(parsed.node_text(&child).to_owned());
        }
    }
}

/// The write statement of an lvalue occurrence, and whether it is a plain `=`.
/// Only a whole `expression_statement` holding one assignment qualifies: a chained
/// inner target, a walrus, loop/`with`/`except` target, import or `x: int` is no write.
/// Tuple/list targets write every element when the statement completes; an abnormal
/// completion leaves the region, so each element is an unconditional write.
pub(super) fn classify_write<'a>(node: Node<'a>, span: (usize, usize)) -> Option<(Node<'a>, bool)> {
    let mut node = node.parent()?;
    loop {
        match node.kind() {
            "pattern_list" | "tuple_pattern" | "list_pattern" | "list_splat_pattern" => {}
            "assignment" | "augmented_assignment" => {
                let left = node.child_by_field_name("left")?;
                node.child_by_field_name("right")?;
                if !(left.start_byte() <= span.0 && span.1 <= left.end_byte()) {
                    return None;
                }
                let statement = node
                    .parent()
                    .filter(|p| p.kind() == "expression_statement")?;
                return Some((statement, node.kind() == "assignment"));
            }
            _ => return None,
        }
        node = node.parent()?;
    }
}
