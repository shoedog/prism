//! Rust occurrence rows `R-01`, `R-03`, `R-06` (alias-twin lane, slice 2a).
//!
//! `R-01`: an identifier written by `=` or bound by a `let`, fn parameter, closure parameter or a
//! `mut`/`ref`/tuple pattern (wherever that pattern sits: `let`, `for (a, mut u) in ..`,
//! `if let (a, mut u) = ..`, match arms) only rebinds the variable. `R-03`: `*u = ..` writes the
//! referent. `R-06` (fold 1): an identifier whose spelling the file declares as a non-local item
//! (`const`, `static`, any `struct`, enum variant) or imports by an explicit `use` leaf or alias is
//! `Unsupported`, because in a pattern it may match that item instead of binding; glob imports and
//! macro-made items are residuals (a constant pattern performs no write, so suppressing its twin
//! cannot remove a real write).
//! `R-02` (compound assignment on a name) is implemented MORE conservatively than the oracle:
//! always `Unsupported`, so the twin stays (the oracle's binding-write needs a corpus-wide scan
//! that no occurrence-local evidence replaces; `AddAssign` on a `&mut` receiver can write
//! borrowed storage). Field and index targets (`R-04`) need reference evidence and are
//! `Unsupported` here. Everything else is `Unsupported`: struct and tuple-struct pattern leaves and
//! destructuring-assignment leaves; a plain identifier directly inside an `if let` or `for` pattern
//! (`Some(e)`, `for i in ..`) produces no occurrence span at all, so it has no twin to suppress.
use super::shared::{field_in_parent, is_plain_ident, span_node, walk_owner};
use super::{Classified, Disposition};
use crate::ast::{ParsedFile, PathSpan};
use std::collections::BTreeSet;
use tree_sitter::Node;

const fn done(disposition: Disposition, rule: &'static str) -> Classified {
    Classified { disposition, rule }
}

/// Classify with the whole file's non-local item names (for tests and parity checks).
#[cfg(test)]
pub(super) fn classify(parsed: &ParsedFile, span: &PathSpan) -> Classified {
    let items = item_names(parsed, &parsed.tree.root_node());
    classify_with(parsed, span, &items)
}

/// Spellings that name a non-local item visible to `owner` and so cannot be a fresh binding
/// where a pattern is matched: `const`, `static`, any `struct` (a unit or tuple struct is a value),
/// enum variants and the leaf or alias of an explicit `use` item (`use a::b;`, `use a::{c as d};`).
/// Module-level items come from the module chain without descending into function bodies; items
/// declared in every enclosing block on the way to the module root are added (`fn outer() { const u:
/// i32 = 1; fn inner(..) { .. } }`), and the owner's own subtree is walked in full (local items). Glob imports (`use m::*;`) and items made
/// by macros are residuals: a constant pattern performs no write, so suppressing its twin cannot
/// remove a real write (the row is a contract fix, not an output fix).
pub(super) fn item_names(parsed: &ParsedFile, owner: &Node<'_>) -> BTreeSet<String> {
    fn use_leaves(parsed: &ParsedFile, node: Node<'_>, out: &mut BTreeSet<String>) {
        match node.kind() {
            "identifier" => {
                out.insert(parsed.node_text(&node).to_string());
            }
            "scoped_identifier" => {
                if let Some(name) = node.child_by_field_name("name") {
                    out.insert(parsed.node_text(&name).to_string());
                }
            }
            "use_as_clause" => {
                if let Some(alias) = node.child_by_field_name("alias") {
                    out.insert(parsed.node_text(&alias).to_string());
                }
            }
            "use_list" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    use_leaves(parsed, child, out);
                }
            }
            "scoped_use_list" => {
                if let Some(list) = node.child_by_field_name("list") {
                    use_leaves(parsed, list, out);
                }
            }
            _ => {}
        }
    }
    let mut out = BTreeSet::new();
    let mut visit = |node: Node<'_>| match node.kind() {
        "const_item" | "static_item" | "struct_item" | "enum_variant" => {
            if let Some(name) = node.child_by_field_name("name") {
                out.insert(parsed.node_text(&name).to_string());
            }
        }
        "use_declaration" => {
            if let Some(arg) = node.child_by_field_name("argument") {
                use_leaves(parsed, arg, &mut out);
            }
        }
        _ => {}
    };
    // Module chain: source file, module bodies, enum bodies (no function bodies).
    let mut stack = vec![parsed.tree.root_node()];
    while let Some(node) = stack.pop() {
        visit(node);
        if matches!(
            node.kind(),
            "source_file" | "mod_item" | "declaration_list" | "enum_item" | "enum_variant_list"
        ) {
            let mut cursor = node.walk();
            stack.extend(node.children(&mut cursor));
        }
    }
    // Enclosing function blocks: items and imports declared beside the owner or above it.
    let mut ancestor = owner.parent();
    while let Some(a) = ancestor {
        if matches!(a.kind(), "block" | "declaration_list") {
            let mut stack: Vec<Node<'_>> = Vec::new();
            let mut cursor = a.walk();
            stack.extend(a.children(&mut cursor));
            while let Some(n) = stack.pop() {
                visit(n);
                if matches!(n.kind(), "enum_item" | "enum_variant_list") {
                    let mut c = n.walk();
                    stack.extend(n.children(&mut c));
                }
            }
        }
        ancestor = a.parent();
    }
    walk_owner(owner, visit);
    out
}

pub(super) fn classify_with(
    parsed: &ParsedFile,
    span: &PathSpan,
    items: &BTreeSet<String>,
) -> Classified {
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
        if items.contains(text) {
            // A pattern identifier spelled like a constant, static, unit struct, variant or
            // explicitly imported name may match that item instead of binding (R-06).
            return done(Disposition::Unsupported, "R-06");
        }
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
            // Comments are extras between tokens: adjacency is tested on the non-comment tokens.
            let kids: Vec<Node<'_>> = node
                .children(&mut cursor)
                .filter(|c| !matches!(c.kind(), "line_comment" | "block_comment"))
                .collect();
            for (i, kid) in kids.iter().enumerate() {
                if kid.kind() != "identifier" {
                    continue;
                }
                let prev = i.checked_sub(1).map(|p| kids[p].kind());
                let next = kids.get(i + 1).map(|n| n.kind());
                // `name = ..` inside a macro is a named format argument or a macro-body assignment.
                if matches!(prev, Some(".") | Some("::")) || matches!(next, Some(":") | Some("=")) {
                    out.insert(parsed.node_text(kid).to_string());
                }
            }
        }
        "reference_expression" => {
            // `&mut <target>` in any position (call argument included) can write the target.
            let mut cursor = node.walk();
            let is_mut = node
                .children(&mut cursor)
                .any(|c| c.kind() == "mutable_specifier");
            if let (true, Some(value)) = (is_mut, node.child_by_field_name("value")) {
                if let Some(root) = expr_root(parsed, value) {
                    out.insert(root);
                }
            }
        }
        "macro_invocation" => {
            // `addr_of_mut!(o)` (any path ending in it) takes a raw mutable pointer to the target.
            let name = node.child_by_field_name("macro").map(|m| {
                let t = parsed.node_text(&m);
                t.rsplit("::").next().unwrap_or(t).trim().to_string()
            });
            if name.as_deref() == Some("addr_of_mut") {
                let mut cursor = node.walk();
                let tree = node
                    .children(&mut cursor)
                    .find(|c| c.kind() == "token_tree");
                if let Some(tree) = tree {
                    let mut cursor = tree.walk();
                    let first = tree
                        .children(&mut cursor)
                        .find(|c| c.kind() == "identifier");
                    if let Some(first) = first {
                        out.insert(parsed.node_text(&first).to_string());
                    }
                }
            }
        }
        "let_declaration" | "let_condition" | "match_expression" => {
            // A `ref mut` binding initialised from, or matched against, the target (`let`, `if let`,
            // `while let`, let chains).
            let source = if matches!(node.kind(), "let_declaration" | "let_condition") {
                node.child_by_field_name("pattern")
                    .zip(node.child_by_field_name("value"))
            } else {
                node.child_by_field_name("body")
                    .zip(node.child_by_field_name("value"))
            };
            if let Some((patterns, value)) = source {
                if has_ref_mut(patterns) {
                    if let Some(root) = expr_root(parsed, value) {
                        out.insert(root);
                    }
                }
            }
        }
        _ => {}
    });
    out
}

/// The root variable of a place expression (`o`, `o.f`, `o[i]`, `(o)`, `*o`).
fn expr_root(parsed: &ParsedFile, mut node: Node<'_>) -> Option<String> {
    loop {
        match node.kind() {
            "identifier" => return Some(parsed.node_text(&node).to_string()),
            "field_expression" => node = node.child_by_field_name("value")?,
            "index_expression"
            | "parenthesized_expression"
            | "unary_expression"
            | "reference_expression" => {
                node = node
                    .child_by_field_name("value")
                    .or_else(|| node.named_child(0))?;
            }
            _ => return None,
        }
    }
}

/// Does the pattern subtree hold `ref mut x` (a `ref_pattern` around a `mut_pattern`)?
fn has_ref_mut(pattern: Node<'_>) -> bool {
    let mut found = false;
    walk_owner(&pattern, |n| {
        if n.kind() == "ref_pattern" {
            let mut cursor = n.walk();
            found |= n.children(&mut cursor).any(|c| c.kind() == "mut_pattern");
        }
    });
    found
}
