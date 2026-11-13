//! C rules U and D: regions, entry detection and the write certificates. Moved verbatim out of
//! `c_prune.rs` (#820).
use super::c_prune::COwner;
use super::c_prune_resolve::{declared, is_plain_declarator_chain, resolve};
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use tree_sitter::Node;

/// Region: a `compound_statement` that is not a switch body, or a `case_statement` that is a
/// direct child of a switch body.
pub(super) fn is_region(node: Node<'_>) -> bool {
    match node.kind() {
        "compound_statement" => node.parent().is_none_or(|p| p.kind() != "switch_statement"),
        "case_statement" => node
            .parent()
            .is_some_and(|b| b.kind() == "compound_statement" && is_switch_body(b)),
        _ => false,
    }
}

pub(super) fn is_switch_body(compound: Node<'_>) -> bool {
    compound
        .parent()
        .is_some_and(|p| p.kind() == "switch_statement")
}

/// A label, or a `case` that is not a direct child of a switch body, starts in
/// `[from, to)` at any depth: control can enter the window there.
pub(super) fn entry_in_range(node: Node<'_>, from: usize, to: usize) -> bool {
    if node.end_byte() <= from || node.start_byte() >= to {
        return false;
    }
    let inside = node.start_byte() >= from && node.start_byte() < to;
    match node.kind() {
        "labeled_statement" if inside => return true,
        "case_statement" if inside => {
            let direct = node
                .parent()
                .is_some_and(|b| b.kind() == "compound_statement" && is_switch_body(b));
            if !direct {
                return true;
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if entry_in_range(child, from, to) {
            return true;
        }
    }
    false
}

/// The write statement of an lvalue occurrence and whether it is a plain `=` / declarator
/// write. The statement is the node whose parent must be the region for a dominating write:
/// the `expression_statement` of a bare assignment, the `declaration` of an initialised
/// declarator; every other write (nested assignment, no initializer) gets a node that is
/// never a region child.
pub(super) fn classify_write<'a>(
    parsed: &ParsedFile,
    node: Node<'a>,
    owner: &COwner,
) -> Option<(Node<'a>, bool)> {
    let _ = owner;
    let parent = node.parent()?;
    // Declarator position: identifier or pointer/parenthesized chain.
    if matches!(
        node.kind(),
        "identifier" | "pointer_declarator" | "parenthesized_declarator"
    ) {
        let mut top = node;
        while let Some(p) = top.parent() {
            if matches!(p.kind(), "pointer_declarator" | "parenthesized_declarator") {
                top = p;
            } else {
                break;
            }
        }
        // The span must be the whole top declarator chain (or an identifier inside it that
        // the chain's top has as its only content).
        if top.id() == node.id() || node.kind() != "identifier" {
            let p = top.parent()?;
            match p.kind() {
                "init_declarator" if p.child_by_field_name("declarator")?.id() == top.id() => {
                    let decl = p.parent()?;
                    if decl.kind() == "declaration" {
                        // C++: a condition declaration is read as the condition itself.
                        if is_condition_declaration(decl) {
                            return None;
                        }
                        return Some((decl, true));
                    }
                }
                "declaration" => {
                    if is_condition_declaration(p) {
                        return None;
                    }
                    // No initializer: a write that never dominates.
                    return Some((top, true));
                }
                _ => {}
            }
        }
    }
    if node.kind() == "identifier" && parent.kind() == "assignment_expression" {
        let left = parent.child_by_field_name("left")?;
        if left.id() != node.id() {
            return None;
        }
        let plain = parent
            .child_by_field_name("operator")
            .is_some_and(|op| parsed.node_text(&op) == "=");
        let statement = match parent.parent() {
            Some(s) if s.kind() == "expression_statement" => s,
            _ => parent,
        };
        return Some((statement, plain));
    }
    None
}

/// `if (int v = f())`: the declaration sits directly in a `condition_clause`, where the declared
/// variable is also the tested value (not a plain write; never a candidate).
fn is_condition_declaration(decl: Node<'_>) -> bool {
    decl.parent()
        .is_some_and(|p| p.kind() == "condition_clause")
}

/// The variable-write certificate of a D candidate (F1): the raw span is an identifier (or
/// a declarator chain around one) spelled as the path's base, in the `left` of an assignment
/// (any operator), in a declarator position of a declaration, or a parameter. Returns the
/// binding's declarator byte start.
pub(super) fn certified_candidate(
    parsed: &ParsedFile,
    start: usize,
    end: usize,
    path: &AccessPath,
) -> Option<usize> {
    let node = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(start, end)?;
    if node.start_byte() != start || node.end_byte() != end {
        return None;
    }
    let d = declared(node)?;
    if d.exotic
        || !path.fields.is_empty()
        || parsed.node_text(&d.ident) != path.base
        || !is_plain_declarator_chain(node)
    {
        return None;
    }
    if node.kind() == "identifier" {
        let parent = node.parent()?;
        if parent.kind() == "assignment_expression" {
            return (parent.child_by_field_name("left")?.id() == node.id())
                .then(|| resolve(parsed, node))
                .flatten()
                .map(|n| n.start_byte());
        }
    }
    // A declarator position: the chain's top sits directly under a declaration,
    // `init_declarator` (as its declarator, never its value) or parameter.
    let mut top = node;
    while let Some(p) = top.parent() {
        if matches!(p.kind(), "pointer_declarator" | "parenthesized_declarator") {
            top = p;
        } else {
            break;
        }
    }
    let p = top.parent()?;
    let ok = match p.kind() {
        "init_declarator" => {
            p.child_by_field_name("declarator")
                .is_some_and(|d| d.id() == top.id())
                && !p.parent().is_some_and(is_condition_declaration)
        }
        "declaration" => !is_condition_declaration(p),
        "parameter_declaration" => true,
        _ => false,
    };
    ok.then(|| d.ident.start_byte())
}

/// The identifier a write span names: the span itself, or the identifier at the end of a
/// pointer/parenthesized declarator chain.
pub(super) fn write_target_identifier(node: Node<'_>) -> Option<Node<'_>> {
    let d = declared(node)?;
    (!d.exotic).then_some(d.ident)
}
