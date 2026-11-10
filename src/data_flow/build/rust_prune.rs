//! Rust facts for rules U and D (#630 slice 5, Rust). Removal-only, lexical.
//!
//! Owners are `function_item`s. Closures, `async`/`gen` blocks (the language's callable
//! boundaries) and nested `fn` items are nested callables: a use inside one gets no proof and
//! D keeps the defs inside one (U can still drop a nested def's edge into an outer
//! write-only line). Out of model, as for `Exact`: `unsafe` raw-pointer writes, interior
//! mutability (`Cell`, `RefCell`), `&mut` aliases of a plain local and macros that expand
//! to a write or a `let` of the name.
//!
//! D for a plain name drops pairs whose own storage the dominating statement provably
//! overwrites. D never proves a field path (`s.f`): a `&mut`/`Box` base or a prefix of the
//! path can be re-pointed and restored, so identical spelling is not identical storage; U
//! stays for field paths. D also keeps (a) a name mutably borrowed anywhere in the owner
//! ([`mutably_borrowed`]: `&mut x`, `&raw mut x`, `ref mut`, token trees included) and (b) a
//! pair whose use line holds an occurrence of the name inside a macro token tree (a macro
//! can generate a local macro retaining a binding). U and D are both off for an owner that
//! defines a `macro_rules!` anywhere ([`has_local_macro`]: a local macro reads a local by
//! definition-site hygiene while the line-anchored use sits at the invocation). `self` is
//! not an `identifier` node in tree-sitter-rust, so `self.*` paths are never proven.
//!
//! Out of model, as in Go design §2.2 and as for `Exact`: a writer prism cannot see (a
//! `&mut self` method called with auto-ref, `clone_from`, interior mutability, `unsafe` raw
//! writes, a macro expanding to a write). Such a writer can bring back the value of a dropped
//! def; that loss is accepted. Per the owner's #634 rule the survivor-based fallback cap lets
//! a formerly capped Rust owner regain its surviving edges when pruning brings it under the
//! cap, so removal-only is not strict for such owners (disclosed, as for Go, Python and JS).
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use std::collections::BTreeSet;
use tree_sitter::Node;

/// Callables that run elsewhere and so are never proven (added to the language's
/// closure/async/gen boundaries).
pub(super) const NESTED_ITEMS: [&str; 1] = ["function_item"];

/// Single-entry region: control enters a `block` only at its first statement. `break 'a`
/// out of a labeled block and `?`/`return` only leave forward; a loop re-enters at the top.
pub(super) fn is_region(kind: &str) -> bool {
    kind == "block"
}

/// The write statement of an lvalue occurrence, and whether it is a plain `=` / `let`.
/// Only a statement that is a direct child of a block qualifies: `let x = ..;` with an
/// identifier pattern, a value and no `else`, or an `expression_statement` holding one
/// `=` / compound assignment whose left side is exactly the span. A statement carrying an
/// attribute (`#[cfg(..)]`) may not exist at all, so it is no write.
pub(super) fn classify_write<'a>(node: Node<'a>, span: (usize, usize)) -> Option<(Node<'a>, bool)> {
    let parent = node.parent()?;
    let (statement, plain) = match parent.kind() {
        "let_declaration" => {
            let pattern = parent.child_by_field_name("pattern")?;
            parent.child_by_field_name("value")?;
            if pattern.id() != node.id() || parent.child_by_field_name("alternative").is_some() {
                return None;
            }
            (parent, true)
        }
        "assignment_expression" | "compound_assignment_expr" => {
            let left = parent.child_by_field_name("left")?;
            parent.child_by_field_name("right")?;
            if !(left.start_byte() == span.0 && span.1 == left.end_byte()) {
                return None;
            }
            let statement = parent
                .parent()
                .filter(|p| p.kind() == "expression_statement")?;
            (statement, parent.kind() == "assignment_expression")
        }
        _ => return None,
    };
    if statement.parent()?.kind() != "block" || has_attribute(statement) {
        return None;
    }
    Some((statement, plain))
}

/// An attribute (`#[cfg(..)]`, `#[allow]`) directly before the statement.
fn has_attribute(statement: Node<'_>) -> bool {
    let mut prev = statement.prev_sibling();
    while let Some(sibling) = prev {
        match sibling.kind() {
            "attribute_item" => return true,
            "line_comment" | "block_comment" => prev = sibling.prev_sibling(),
            _ => return false,
        }
    }
    false
}

/// The path an lvalue node spells exactly: an identifier, or a chain of plain field
/// accesses on one (`s.f`; `self` is never proven, see the module doc). Indexing, dereference, calls and tuple indices
/// prove nothing.
pub(super) fn pure_path(parsed: &ParsedFile, node: Node<'_>) -> Option<AccessPath> {
    match node.kind() {
        "identifier" | "self" => Some(AccessPath::simple(parsed.node_text(&node))),
        "field_expression" => {
            let mut path = pure_path(parsed, node.child_by_field_name("value")?)?;
            let field = node.child_by_field_name("field")?;
            if field.kind() != "field_identifier" {
                return None;
            }
            path.fields.push(parsed.node_text(&field).to_owned());
            Some(path)
        }
        _ => None,
    }
}

/// An owner that defines a `macro_rules!` anywhere (nested scopes included): D keeps every pair.
pub(super) fn has_local_macro(owner: Node<'_>) -> bool {
    if owner.kind() == "macro_definition" {
        return true;
    }
    let mut cursor = owner.walk();
    let found = owner
        .named_children(&mut cursor)
        .any(|child| has_local_macro(child));
    found
}

/// Local names mutably borrowed anywhere in the owner (nested callables and macro token
/// trees included): `&mut x`, `&mut x.f`, `&mut *x`, `&raw mut x`, and a `ref mut` binding
/// of a place rooted at `x` (`let ref mut r = x`, `match x { ref mut r => .. }`, `if let
/// Some(ref mut r) = x`). In a token tree every identifier beside a `mut` token counts
/// (`dbg!(swap(&mut x, ..))`, `ref mut x`). D keeps such a name: the borrow can re-point or
/// restore it through a writer prism records no def for (the analogue of Go rule S's
/// address-taken set). Auto-referenced method receivers are not seen (see the module doc).
pub(super) fn mutably_borrowed(parsed: &ParsedFile, owner: Node<'_>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    scan_borrows(parsed, owner, &mut out);
    out
}

fn scan_borrows(parsed: &ParsedFile, node: Node<'_>, out: &mut BTreeSet<String>) {
    let mut cursor = node.walk();
    match node.kind() {
        "reference_expression" => {
            if node
                .children(&mut cursor)
                .any(|c| c.kind() == "mutable_specifier")
            {
                if let Some(value) = node.child_by_field_name("value") {
                    mark_roots(parsed, value, out);
                }
            }
        }
        "ref_pattern" | "field_pattern" if has_ref_mut(node) => {
            let mut up = node.parent();
            while let Some(holder) = up {
                if matches!(
                    holder.kind(),
                    "let_declaration" | "let_condition" | "match_expression" | "for_expression"
                ) {
                    if let Some(value) = holder.child_by_field_name("value") {
                        mark_roots(parsed, value, out);
                    }
                    break;
                }
                up = holder.parent();
            }
        }
        "token_tree" => {
            let direct: Vec<Node<'_>> = node.children(&mut cursor).collect();
            if direct.iter().any(|c| c.kind() == "mutable_specifier") {
                for child in direct.iter().filter(|c| c.kind() == "identifier") {
                    out.insert(parsed.node_text(child).to_owned());
                }
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        scan_borrows(parsed, child, out);
    }
}

/// A `ref` pattern holding `mut` (`ref mut r`, `ref mut` in a struct-pattern field).
fn has_ref_mut(node: Node<'_>) -> bool {
    fn contains_mut(n: Node<'_>) -> bool {
        let mut cursor = n.walk();
        let found = n
            .children(&mut cursor)
            .any(|c| c.kind() == "mutable_specifier" || contains_mut(c));
        found
    }
    if node.kind() == "field_pattern" {
        let mut cursor = node.walk();
        let has_ref = node.children(&mut cursor).any(|c| c.kind() == "ref");
        return has_ref && contains_mut(node);
    }
    contains_mut(node)
}

/// The identifier a place expression is rooted at (through parentheses, fields, indexing,
/// dereference and `?`); anything else roots nothing.
fn mark_roots(parsed: &ParsedFile, value: Node<'_>, out: &mut BTreeSet<String>) {
    let mut node = value;
    loop {
        match node.kind() {
            "identifier" => {
                out.insert(parsed.node_text(&node).to_owned());
                return;
            }
            "field_expression" => match node.child_by_field_name("value") {
                Some(next) => node = next,
                None => return,
            },
            "parenthesized_expression"
            | "index_expression"
            | "unary_expression"
            | "try_expression" => match node.named_child(0) {
                Some(next) => node = next,
                None => return,
            },
            _ => return,
        }
    }
}

/// Names with an occurrence inside a macro token tree, with the line: D keeps a use line
/// holding one (a macro can generate a local macro that retains a binding, or a binding
/// the line-anchored use cannot tell apart).
pub(super) fn in_macro_tokens(ident: Node<'_>) -> bool {
    let mut up = ident.parent();
    while let Some(node) = up {
        if node.kind() == "token_tree" {
            return node
                .parent()
                .is_some_and(|p| p.kind() == "macro_invocation" || p.kind() == "token_tree");
        }
        up = node.parent();
    }
    false
}
