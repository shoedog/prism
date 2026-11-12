//! Python occurrence rows `Y-01`, `Y-02` (alias-twin lane, slice 2a).
//!
//! `Y-01`: a name bound by `=` (plain, annotated, chained), a pattern leaf of an assignment or `for`
//! statement, a `with`/`except .. as` target or a walrus only rebinds the variable.
//! `Y-02`: `u += rhs` is `Unsupported` (`__iadd__` and list `+=` mutate in place). Attribute and
//! subscript targets (`Y-03`) are `StorageWrite`. Comprehension targets and everything else are
//! `Unsupported`.
use super::shared::{field_in_parent, is_plain_ident, span_node, walk_owner};
use super::{Classified, Disposition};
use crate::ast::{ParsedFile, PathSpan};
use std::collections::BTreeSet;
use tree_sitter::Node;

const fn done(disposition: Disposition, rule: &'static str) -> Classified {
    Classified { disposition, rule }
}

pub(super) fn classify(parsed: &ParsedFile, span: &PathSpan) -> Classified {
    let unrecognised = done(Disposition::Unsupported, "X-UNRECOGNISED");
    let Some(node) = span_node(parsed, span) else {
        return unrecognised;
    };
    let Some(parent) = node.parent() else {
        return unrecognised;
    };
    let field = field_in_parent(node).unwrap_or("");
    let text = parsed.node_text(&node).trim();
    if matches!(node.kind(), "attribute" | "subscript") {
        return match (parent.kind(), field) {
            ("assignment", "left") | ("augmented_assignment", "left") => {
                done(Disposition::StorageWrite, "Y-03")
            }
            _ => unrecognised,
        };
    }
    if !is_plain_ident(text) {
        return unrecognised;
    }
    match (node.kind(), parent.kind(), field) {
        ("identifier", "augmented_assignment", "left") => done(Disposition::Unsupported, "Y-02"),
        ("identifier", "assignment", "left") | ("identifier", "for_statement", "left") => {
            done(Disposition::BindingWrite, "Y-01")
        }
        ("identifier", "named_expression", "name") => done(Disposition::BindingWrite, "Y-01"),
        ("as_pattern_target", "as_pattern", _) => done(Disposition::BindingWrite, "Y-01"),
        (
            "identifier",
            "pattern_list" | "tuple_pattern" | "list_pattern" | "list_splat_pattern",
            _,
        ) => {
            if pattern_root_binds(parent) {
                done(Disposition::BindingWrite, "Y-01")
            } else {
                unrecognised
            }
        }
        _ => unrecognised,
    }
}

/// A pattern leaf binds when its pattern chain ends in an assignment or `for` target.
fn pattern_root_binds(mut pattern: Node<'_>) -> bool {
    while matches!(
        pattern.kind(),
        "pattern_list"
            | "tuple_pattern"
            | "list_pattern"
            | "list_splat_pattern"
            | "parenthesized_pattern"
    ) {
        let Some(parent) = pattern.parent() else {
            return false;
        };
        let field = field_in_parent(pattern).unwrap_or("");
        if matches!(parent.kind(), "assignment" | "for_statement") && field == "left" {
            return true;
        }
        pattern = parent;
    }
    false
}

/// Spellings the occurrence extraction can mistake for a variable inside `owner`: attribute names
/// (`x.o`) and keyword-argument names (`f(o=1)`).
pub(super) fn member_name_tokens(parsed: &ParsedFile, owner: &Node<'_>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    walk_owner(owner, |node| match node.kind() {
        "attribute" => {
            if let Some(attr) = node.child_by_field_name("attribute") {
                out.insert(parsed.node_text(&attr).to_string());
            }
        }
        "keyword_argument" => {
            if let Some(name) = node.child_by_field_name("name") {
                out.insert(parsed.node_text(&name).to_string());
            }
        }
        _ => {}
    });
    out
}
