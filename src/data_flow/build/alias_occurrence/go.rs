//! Go occurrence rows `G-01`, `G-02` (alias-twin lane, slice 2a).
//!
//! `G-01`: a name written by `=` (also compound), `:=`, `var`, a range clause or a receive
//! statement only rebinds the variable (Go has no operator overloading and no variable
//! references), so it is a binding write. `G-02`: `*u = ..` writes the pointee. `G-03`/`G-04`
//! (selector and index targets) need pointer/slice/map declaration evidence the classifier does
//! not read: they are `Unsupported` here (the oracle says `storage-write` when proven), which keeps
//! the twin exactly like `storage-write` does. Everything else is `Unsupported`.
use super::shared::{field_in_parent, is_plain_ident, span_node, walk_owner};
use super::{Classified, Disposition};
use crate::ast::{ParsedFile, PathSpan};
use std::collections::BTreeSet;
use tree_sitter::Node;

const fn done(disposition: Disposition, rule: &'static str) -> Classified {
    Classified { disposition, rule }
}

/// The `(parent kind, field)` of the statement slot an lvalue sits in, looking through one
/// `expression_list` (`a, b := f()` and the single-element list around `u`).
fn slot<'t>(node: Node<'t>) -> Option<(&'static str, &'static str)> {
    let mut outer = node;
    if let Some(parent) = node.parent() {
        if parent.kind() == "expression_list" {
            outer = parent;
        }
    }
    let parent = outer.parent()?;
    Some((parent.kind(), field_in_parent(outer).unwrap_or("")))
}

pub(super) fn classify(parsed: &ParsedFile, span: &PathSpan) -> Classified {
    let unrecognised = done(Disposition::Unsupported, "X-UNRECOGNISED");
    let Some(node) = span_node(parsed, span) else {
        return unrecognised;
    };
    // The span's node may be the single-element list itself: judge its element.
    let leaf = if node.kind() == "expression_list" && node.named_child_count() == 1 {
        node.named_child(0).unwrap_or(node)
    } else {
        node
    };
    let text = parsed.node_text(&leaf).trim();
    let binding_slot = |s: (&str, &str)| {
        matches!(
            s,
            ("assignment_statement", "left")
                | ("short_var_declaration", "left")
                | ("range_clause", "left")
                | ("receive_statement", "left")
                | ("var_spec", "name")
        )
    };
    let Some(slot) = slot(node) else {
        return unrecognised;
    };
    if leaf.kind() == "identifier" && is_plain_ident(text) && binding_slot(slot) {
        return done(Disposition::BindingWrite, "G-01");
    }
    if text.starts_with('*') && slot == ("assignment_statement", "left") {
        return done(Disposition::StorageWrite, "G-02");
    }
    unrecognised
}

/// Spellings the occurrence extraction can mistake for a variable inside `owner`: selector field
/// and method names, keyed composite-literal keys and statement labels (refusal R1 analog).
pub(super) fn member_name_tokens(parsed: &ParsedFile, owner: &Node<'_>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    walk_owner(owner, |node| match node.kind() {
        "field_identifier" | "label_name" => {
            out.insert(parsed.node_text(&node).to_string());
        }
        "keyed_element" => {
            if let Some(key) = node.named_child(0) {
                let text = parsed.node_text(&key).trim();
                if is_plain_ident(text) {
                    out.insert(text.to_string());
                }
            }
        }
        _ => {}
    });
    out
}
