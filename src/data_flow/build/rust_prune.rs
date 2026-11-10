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
//! overwrites. D for a field path `b.f..` additionally needs a stable base `b`
//! ([`stable_bases`]): a `&mut`/`Box` base can be re-pointed and restored, or moved out
//! and back, so identical spelling is not identical storage. `self` is not an
//! `identifier` node in tree-sitter-rust and the occurrence collector never sees it, so
//! `self.*` paths are never proven (U and D keep them); that is a recall limit.
//! D never runs in an owner holding a `macro_rules!` definition (a macro defined inside
//! the owner reads a local by definition-site hygiene while the line-anchored use sits at
//! the invocation); a macro defined outside the owner cannot name the owner's locals, so
//! that case is left alone. U is unaffected: macro tokens on a line already make it not
//! write-only. Per the owner's #634 rule the survivor-based fallback cap lets a formerly
//! capped Rust owner regain its surviving edges when pruning brings it under the cap, so
//! removal-only is not strict for such owners (disclosed, as for Go, Python and JS).
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};
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

/// Tokens that make a macro token tree a possible binder or writer of an identifier in it.
const BINDING_TOKENS: [&str; 18] = [
    "let", "for", "match", "|", "=>", "=", "mut", "ref", "+=", "-=", "*=", "/=", "%=", "^=", "&=",
    "|=", "<<=", ">>=",
];

/// Local names whose field paths D may prove (`s.f`): the name is bound exactly once in the
/// owner, nested callables and macro token trees included, as an owner formal or one
/// identifier-pattern `let`, and is never assigned (`s = ..`, `s op= ..`, a destructuring
/// assignment) nor mutably borrowed (`&mut s`, which can re-point it through the borrow).
/// Any pattern binding (`if let`/`while let`/`match`/`for`/closure parameter/tuple or struct
/// pattern), a second `let`, a same-named parameter of a nested closure or `fn`, or a nested
/// item of that name makes it unstable. Method calls with an auto-referenced receiver
/// (`s.reset()` with `impl Reset for Box<S>`) are out of model, as for `Exact`.
pub(super) fn stable_bases(parsed: &ParsedFile, owner: Node<'_>) -> BTreeSet<String> {
    let mut state = Bases::default();
    scan(parsed, owner, owner.id(), &mut state);
    state
        .simple_once
        .into_iter()
        .filter(|name| state.unstable.get(name).is_none() && state.count.get(name) == Some(&1))
        .collect()
}

#[derive(Default)]
struct Bases {
    count: BTreeMap<String, usize>,
    simple_once: BTreeSet<String>,
    unstable: BTreeMap<String, ()>,
}

fn is_pattern_kind(kind: &str) -> bool {
    kind.ends_with("_pattern") || kind == "field_pattern" || kind == "mutable_specifier"
}

fn scan(parsed: &ParsedFile, node: Node<'_>, owner_id: usize, state: &mut Bases) {
    let kind = node.kind();
    if kind == "identifier" || kind == "shorthand_field_identifier" {
        note_identifier(parsed, node, owner_id, state);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        scan(parsed, child, owner_id, state);
    }
}

fn note_identifier(parsed: &ParsedFile, ident: Node<'_>, owner_id: usize, state: &mut Bases) {
    let name = parsed.node_text(&ident).to_owned();
    if let Some(simple) = binding_form(ident, owner_id) {
        *state.count.entry(name.clone()).or_default() += 1;
        if simple {
            state.simple_once.insert(name.clone());
        }
    }
    if assigned_or_reborrowed(ident) || in_binding_macro(ident) {
        state.unstable.insert(name, ());
    }
}

/// `Some(simple)` when the identifier occurrence binds a name (or may, conservatively);
/// `simple` is true only for an owner formal or an identifier-pattern `let`.
fn binding_form(ident: Node<'_>, owner_id: usize) -> Option<bool> {
    let parent = ident.parent()?;
    // A nested item named like the identifier shadows it for the whole item scope.
    if matches!(
        parent.kind(),
        "function_item" | "const_item" | "static_item"
    ) && parent.child_by_field_name("name") == Some(ident)
    {
        return Some(false);
    }
    let mut top = ident;
    while let Some(up) = top.parent() {
        if is_pattern_kind(up.kind()) && up.kind() != "match_pattern" {
            top = up;
        } else {
            break;
        }
    }
    if ident.kind() == "shorthand_field_identifier" && parent.kind() != "field_pattern" {
        return None;
    }
    let holder = top.parent()?;
    let simple_pattern = top.id() == ident.id()
        || (top.kind() == "mut_pattern"
            && top.named_child_count() == 2
            && top.named_child(1) == Some(ident));
    match holder.kind() {
        "let_declaration" if holder.child_by_field_name("pattern") == Some(top) => {
            let plain = simple_pattern
                && holder.child_by_field_name("value").is_some()
                && holder.child_by_field_name("alternative").is_none();
            Some(plain)
        }
        "parameter" if holder.child_by_field_name("pattern") == Some(top) => {
            let formal = holder
                .parent()
                .filter(|list| list.kind() == "parameters")
                .and_then(|list| list.parent())
                .is_some_and(|f| f.id() == owner_id);
            Some(formal && simple_pattern)
        }
        "let_condition" | "for_expression"
            if holder.child_by_field_name("pattern") == Some(top) =>
        {
            Some(false)
        }
        "match_pattern" if holder.child_by_field_name("condition") != Some(top) => Some(false),
        "closure_parameters" => Some(false),
        _ => None,
    }
}

/// `s = ..`, `s op= ..` (also through tuple/struct/call destructuring assignment) and
/// `&mut s`.
fn assigned_or_reborrowed(ident: Node<'_>) -> bool {
    if ident.kind() != "identifier" {
        return false;
    }
    let mut node = ident;
    while let Some(parent) = node.parent() {
        match parent.kind() {
            "assignment_expression" | "compound_assignment_expr" => {
                return parent.child_by_field_name("left") == Some(node)
            }
            "reference_expression" => {
                let mut cursor = parent.walk();
                return parent
                    .children(&mut cursor)
                    .any(|c| c.kind() == "mutable_specifier");
            }
            "parenthesized_expression"
            | "tuple_expression"
            | "array_expression"
            | "struct_expression"
            | "field_initializer_list"
            | "field_initializer"
            | "shorthand_field_initializer"
            | "call_expression"
            | "arguments" => node = parent,
            _ => return false,
        }
    }
    false
}

/// An occurrence inside a macro token tree whose tokens can bind or write (`let`, `for`,
/// `|`, `=>`, `=`, `mut`, `ref`, compound assignment): the tree is unparsed, so unstable.
fn in_binding_macro(ident: Node<'_>) -> bool {
    let mut up = ident.parent();
    while let Some(node) = up {
        if node.kind() == "token_tree" {
            let mut cursor = node.walk();
            if node
                .children(&mut cursor)
                .any(|c| BINDING_TOKENS.contains(&c.kind()))
            {
                return true;
            }
        }
        up = node.parent();
    }
    false
}
