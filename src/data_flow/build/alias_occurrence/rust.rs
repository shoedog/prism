//! Rust occurrence rows `R-01`, `R-03` (alias-twin lane, slice 2a).
//!
//! `R-01`: an identifier written by `=` or bound by a `let`, `mut`, `ref`/`&`, tuple pattern,
//! closure or fn parameter only rebinds the variable. `R-03`: `*u = ..` writes the referent.
//! `R-02` (compound assignment on a name) is implemented MORE conservatively than the oracle:
//! always `Unsupported`, so the twin stays (the oracle's binding-write needs a corpus-wide scan
//! that no occurrence-local evidence replaces; `AddAssign` on a `&mut` receiver can write
//! borrowed storage). Field and index targets (`R-04`) need reference evidence and are
//! `Unsupported` here. Everything else (struct and tuple-struct pattern leaves, `for` patterns,
//! destructuring assignment leaves, `if let` bindings) is `Unsupported`.
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
    if parent.kind() == "tuple_struct_pattern" && field == "type" {
        return done(Disposition::Unsupported, "R-05");
    }
    if node.kind() == "identifier" && is_plain_ident(text) {
        match (parent.kind(), field) {
            ("compound_assignment_expr", "left") => {
                return done(Disposition::Unsupported, "R-02");
            }
            ("assignment_expression", "left")
            | ("let_declaration", "pattern")
            | ("parameter", "pattern") => return done(Disposition::BindingWrite, "R-01"),
            ("mut_pattern", _)
            | ("ref_pattern", _)
            | ("tuple_pattern", _)
            | ("closure_parameters", _) => return done(Disposition::BindingWrite, "R-01"),
            _ => {}
        }
    }
    if text.starts_with('*')
        && matches!(
            (parent.kind(), field),
            ("assignment_expression", "left") | ("compound_assignment_expr", "left")
        )
    {
        return done(Disposition::StorageWrite, "R-03");
    }
    unrecognised
}

/// Spellings the occurrence extraction can mistake for a variable inside `owner`: field and
/// method names (`x.o`, `x.o()`, struct-literal and struct-pattern keys), segments of scoped
/// paths (`m::o`, `T::o`) and macro tokens that follow `.`/`::` or precede a single `:`.
pub(super) fn member_name_tokens(parsed: &ParsedFile, owner: &Node<'_>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    walk_owner(owner, |node| match node.kind() {
        "field_identifier" => {
            out.insert(parsed.node_text(&node).to_string());
        }
        "scoped_identifier" | "scoped_type_identifier" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "identifier" {
                    out.insert(parsed.node_text(&child).to_string());
                }
            }
        }
        "token_tree" => {
            let mut cursor = node.walk();
            let kids: Vec<Node<'_>> = node.children(&mut cursor).collect();
            for (i, kid) in kids.iter().enumerate() {
                if kid.kind() != "identifier" {
                    continue;
                }
                let prev = i.checked_sub(1).map(|p| kids[p].kind());
                let next = kids.get(i + 1).map(|n| n.kind());
                if matches!(prev, Some(".") | Some("::")) || next == Some(":") {
                    out.insert(parsed.node_text(kid).to_string());
                }
            }
        }
        _ => {}
    });
    out
}
