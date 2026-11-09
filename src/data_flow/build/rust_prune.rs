//! Rust facts for rules U and D (#630 slice 5, Rust). Removal-only, lexical.
//!
//! Owners are `function_item`s. Closures, `async`/`gen` blocks (the language's callable
//! boundaries) and nested `fn` items are nested callables: a def or use inside one is
//! never proven. Out of model, as for `Exact`: `unsafe` raw-pointer writes, interior
//! mutability (`Cell`, `RefCell`), `&mut` aliases and macros that expand to a write or a
//! `let` of the name. D only drops pairs whose own storage the dominating statement
//! provably overwrites, so an unseen extra writer never resurrects a dropped def.
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
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
/// accesses on one (`s.f`, `self.a.b`). Indexing, dereference, calls and tuple indices
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
