//! Rust facts for rules U and D (#630 slice 5, Rust). Removal-only, lexical.
//!
//! Owners are `function_item`s. Closures, `async`/`gen` blocks (the language's callable
//! boundaries) and nested `fn` items are nested callables: a use inside one gets no proof and
//! D keeps the defs inside one (U can still drop a nested def's edge into an outer
//! write-only line). Out of model, as for `Exact`: `unsafe` raw-pointer writes, interior
//! mutability (`Cell`, `RefCell`), auto-referenced `&mut` receivers and macros that expand
//! to a write or a `let` of the name.
//!
//! D for a plain name drops pairs whose own storage the dominating statement provably
//! overwrites. D never proves a field path (`s.f`): a `&mut`/`Box` base or a prefix of the
//! path can be re-pointed and restored, so identical spelling is not identical storage; U
//! stays for field paths. D also keeps a name mutably borrowed anywhere in the owner
//! ([`mutably_borrowed`]: `&mut x`, `&raw mut x`, `ref mut`, token trees included).
//!
//! Macro hygiene (fold 3, design note option 3b): both rules read unexpanded source, so
//! every read of a local on a line must be witnessed by an identifier token on it. A
//! `macro_rules!` that reads a local by definition-site hygiene breaks that. U and D are off
//! for an owner that defines one ([`has_local_macro`]), for one whose token trees hold the
//! identifier `macro_rules` or invoke `paste!`/`concat_idents!` ([`scan_macros`]), and a name
//! that is an identifier token of an invocation the repo-wide census
//! (`RustMacroCensus`) refuses gets neither rule in the owner (a generator can emit a
//! `macro_rules!` reading it on a line with no token of it). An implicit format capture
//! (`{x}`) is a read for pruning purposes ([`format_captures`], [`statement_lines`]).
//! `self` is not an `identifier` node in tree-sitter-rust, so `self.*` paths are never
//! proven.
//!
//! Out of model, as in Go design §2.2 and as for `Exact`: a writer prism cannot see (a
//! `&mut self` method called with auto-ref, `clone_from`, interior mutability, `unsafe` raw
//! writes, a macro expanding to a write); an attribute proc macro on the owner or an
//! enclosing item (it rewrites the body); an identifier assembled from fragments by an
//! external name-building macro under another name; and the #772-style root collision E2
//! also accepts. Such a writer can bring back the value of a dropped def; that loss is
//! accepted. Per the owner's #634 rule the survivor-based fallback cap lets a formerly
//! capped Rust owner regain its surviving edges when pruning brings it under the cap, so
//! removal-only is not strict for such owners (disclosed, as for Go, Python and JS).
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::call_graph::rust::macro_args::{scan_soup, Invocation, RustMacroCensus, Soup, Verdict};
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

/// An owner that defines a `macro_rules!` anywhere (nested scopes included): U and D are both
/// off for it (`LexicalFacts::new`).
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

/// What the macro invocations of an owner say about U and D (fold 3, design note option 3b).
pub(super) struct OwnerMacros<'a> {
    /// Identifier tokens of every invocation that can emit a `macro_rules!`: U and D leave
    /// these names alone (a generated macro can read one on a line with no token of it).
    pub refused: BTreeSet<String>,
    /// Some token tree holds the identifier `macro_rules`, or `paste!`/`concat_idents!`
    /// builds names: no per-line premise holds, so U and D are off for the owner.
    pub owner_off: bool,
    /// Implicit format captures (`{x}`) found in string literal tokens of macro token trees,
    /// with the literal's node.
    pub captures: Vec<(String, Node<'a>)>,
}

/// Scan every `macro_invocation` of the owner (nested callables and items included): its
/// own path and every `ident ! (..)` spelled in its token tree are judged by the census; one
/// Refused spelling refuses every identifier token of the invocation.
pub(super) fn scan_macros<'a>(
    parsed: &'a ParsedFile,
    owner: Node<'a>,
    census: &RustMacroCensus,
) -> OwnerMacros<'a> {
    let mut out = OwnerMacros {
        refused: BTreeSet::new(),
        owner_off: false,
        captures: Vec::new(),
    };
    scan_node(parsed, owner, census, &mut out);
    out
}

fn is_name_builder(inv: &Invocation) -> bool {
    matches!(inv.spelling.as_str(), "paste" | "concat_idents")
}

fn scan_node<'a>(
    parsed: &'a ParsedFile,
    node: Node<'a>,
    census: &RustMacroCensus,
    out: &mut OwnerMacros<'a>,
) {
    if node.kind() == "macro_invocation" {
        let own = node
            .child_by_field_name("macro")
            .map(|m| Invocation::from_path(parsed.node_text(&m)));
        let mut soup = Soup::default();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "token_tree" {
                scan_soup(parsed, child, &mut soup);
            }
        }
        let spelled: Vec<&Invocation> = own.iter().chain(&soup.invocations).collect();
        out.owner_off |= soup.macro_rules || spelled.iter().any(|i| is_name_builder(i));
        // An unreadable path is refused (fail closed).
        let refused = own.is_none()
            || spelled
                .iter()
                .any(|i| census.verdict(i.qualifier.as_deref(), &i.spelling) == Verdict::Refused);
        for literal in &soup.strings {
            for name in format_captures(parsed.node_text(literal)) {
                out.captures.push((name, *literal));
            }
        }
        if refused {
            out.refused.extend(soup.idents);
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        scan_node(parsed, child, census, out);
    }
}

/// Names an implicit format capture reads in the text of a string literal token: `{name}`,
/// `{name:spec}`, `{name:?}` and the `name$` width/precision arguments of a spec. `{{`,
/// positional `{0}`, empty `{}` and `{:?}` name nothing. Conservative: applied to the
/// literals of every macro, not only the format family.
pub(super) fn format_captures(literal: &str) -> Vec<String> {
    let ident = |s: &str| {
        let s = s.trim();
        let mut chars = s.chars();
        chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
            && chars.all(|c| c.is_alphanumeric() || c == '_')
    };
    let mut out = Vec::new();
    let mut rest = literal;
    while let Some(at) = rest.find('{') {
        rest = &rest[at + 1..];
        if let Some(after) = rest.strip_prefix('{') {
            rest = after;
            continue;
        }
        let Some(end) = rest.find('}') else { break };
        let body = &rest[..end];
        rest = &rest[end..];
        let (name, spec) = body.split_once(':').unwrap_or((body, ""));
        if ident(name) {
            out.push(name.trim().to_owned());
        }
        // `{:width$}`, `{:.prec$}`, `{:>1$}`: an identifier before `$` is a capture too.
        let mut token = String::new();
        for c in spec.chars().chain(std::iter::once(' ')) {
            if c.is_alphanumeric() || c == '_' {
                token.push(c);
            } else {
                if c == '$' && ident(&token) {
                    out.push(token.clone());
                }
                token.clear();
            }
        }
    }
    out
}

/// The lines of the statement holding `node`: the child of the innermost enclosing block
/// (a closure or arm body without a block of its own takes the whole outer statement), so
/// a capture is a read on every line the statement spans.
pub(super) fn statement_lines(node: Node<'_>) -> std::ops::RangeInclusive<usize> {
    let mut statement = node;
    while let Some(parent) = statement.parent() {
        if parent.kind() == "block" {
            break;
        }
        statement = parent;
    }
    statement.start_position().row + 1..=statement.end_position().row + 1
}
