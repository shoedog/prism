//! JavaScript, TypeScript and TSX occurrence rows `S-01..S-05` and the fallback-anchor row `A-01`
//! (alias-twin lane, slice 4c).
//!
//! `S-01`: a name written by `=`, a compound or logical assignment, `++`/`--`, a declarator name,
//! a `for` head or a `catch` parameter only rebinds the variable. `S-02`/`S-03`: a leaf of a
//! declaration or assignment destructuring pattern binds a new name. `S-05`: an uninitialised
//! declarator. `S-04`: a member or subscript target of an assignment writes storage. Everything
//! else (a default value, a read position, `u.x++`, `for (u.x of ..)`) is `Unsupported`.
//!
//! Refusals beyond the table (all keep the twin, none changes the relation): every occurrence
//! inside a formal parameter list (#761: `function f(o, u = o, v = u.x++)` relies on the binding twin
//! to keep `o@1->o@3` off `exact`; option A excludes parameter defaults), and the member-name
//! tokens of R1 ([`member_name_tokens`]).
use super::shared::{field_in_parent, is_plain_ident, span_node, walk_owner};
use super::{Classified, Disposition};
use crate::ast::{ParsedFile, PathSpan};
use std::collections::BTreeSet;
use tree_sitter::Node;

const fn done(disposition: Disposition, rule: &'static str) -> Classified {
    Classified { disposition, rule }
}

const PATTERNS: [&str; 4] = [
    "array_pattern",
    "object_pattern",
    "pair_pattern",
    "rest_pattern",
];

/// True when the node sits in a callable's parameter list (formals, defaults and patterns).
pub(super) fn in_parameter_list(node: Node<'_>) -> bool {
    let mut cur = node;
    while let Some(parent) = cur.parent() {
        match parent.kind() {
            "formal_parameters" => return true,
            "arrow_function" if field_in_parent(cur) == Some("parameter") => return true,
            "statement_block" | "program" | "class_body" => return false,
            _ => {}
        }
        cur = parent;
    }
    false
}

pub(super) fn classify(parsed: &ParsedFile, span: &PathSpan) -> Classified {
    let unrecognised = done(Disposition::Unsupported, "X-UNRECOGNISED");
    let Some(node) = span_node(parsed, span) else {
        return unrecognised;
    };
    if in_parameter_list(node) {
        return done(Disposition::Unsupported, "X-PARAM");
    }
    let Some(parent) = node.parent() else {
        return unrecognised;
    };
    let field = field_in_parent(node).unwrap_or("");
    let text = parsed.node_text(&node).trim();
    if matches!(node.kind(), "member_expression" | "subscript_expression") {
        return match (parent.kind(), field) {
            ("assignment_expression" | "augmented_assignment_expression", "left") => {
                done(Disposition::StorageWrite, "S-04")
            }
            _ => unrecognised,
        };
    }
    if node.kind() == "variable_declarator" {
        return if is_plain_ident(text) {
            done(Disposition::BindingWrite, "S-05")
        } else {
            unrecognised
        };
    }
    if !is_plain_ident(text)
        || !matches!(
            node.kind(),
            "identifier" | "shorthand_property_identifier_pattern"
        )
    {
        return unrecognised;
    }
    match (parent.kind(), field) {
        ("assignment_expression" | "augmented_assignment_expression", "left")
        | ("update_expression", "argument")
        | ("variable_declarator", "name")
        | ("for_in_statement", "left")
        | ("catch_clause", "parameter") => done(Disposition::BindingWrite, "S-01"),
        (kind, _) if PATTERNS.contains(&kind) => {
            if node.kind() == "identifier" && kind == "pair_pattern" && field != "value" {
                return unrecognised;
            }
            match pattern_root(parent) {
                Some(("variable_declarator", "name"))
                | Some(("for_in_statement", "left"))
                | Some(("catch_clause", "parameter")) => done(Disposition::BindingWrite, "S-02"),
                Some(("assignment_expression", "left")) => done(Disposition::BindingWrite, "S-03"),
                _ => unrecognised,
            }
        }
        _ => unrecognised,
    }
}

/// The (parent kind, field) of the outermost pattern above `pattern`.
fn pattern_root(mut pattern: Node<'_>) -> Option<(&'static str, &'static str)> {
    while PATTERNS.contains(&pattern.kind()) {
        let parent = pattern.parent()?;
        let field = field_in_parent(pattern).unwrap_or("");
        if !PATTERNS.contains(&parent.kind()) {
            let kind: &'static str = match parent.kind() {
                "variable_declarator" => "variable_declarator",
                "for_in_statement" => "for_in_statement",
                "catch_clause" => "catch_clause",
                "assignment_expression" => "assignment_expression",
                _ => return None,
            };
            let f: &'static str = match field {
                "name" => "name",
                "left" => "left",
                "parameter" => "parameter",
                _ => return None,
            };
            return Some((kind, f));
        }
        pattern = parent;
    }
    None
}

/// A-01: a fallback anchor sits on the first token of the alias line. It is a binding write only when
/// that token lies inside a declaration, a `for` declaration head or a catch clause with no assignment
/// or expression statement above it.
pub(super) fn anchor_is_declaration(parsed: &ParsedFile, line: usize) -> bool {
    let byte = parsed.line_start_byte(line);
    let src = parsed.source.as_bytes();
    let mut at = byte;
    while at < src.len() && (src[at] == b' ' || src[at] == b'\t') {
        at += 1;
    }
    let Some(mut node) = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(at, at + 1)
    else {
        return false;
    };
    loop {
        match node.kind() {
            "lexical_declaration" | "variable_declaration" => return !in_parameter_list(node),
            "for_in_statement" => return node.child_by_field_name("kind").is_some(),
            "expression_statement" | "assignment_expression" | "statement_block" | "program" => {
                return false
            }
            _ => {}
        }
        let Some(parent) = node.parent() else {
            return false;
        };
        node = parent;
    }
}

/// Spellings the occurrence extraction can mistake for a variable inside `owner`: property names
/// (`x.o`, `{ o: 1 }`, `class { o() {} }`, JSX attributes), object shorthand, labels, type names and
/// import or export specifier names.
pub(super) fn member_name_tokens(parsed: &ParsedFile, owner: &Node<'_>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    walk_owner(owner, |node| match node.kind() {
        "property_identifier"
        | "private_property_identifier"
        | "shorthand_property_identifier"
        | "statement_identifier"
        | "type_identifier" => {
            out.insert(parsed.node_text(&node).trim().to_string());
        }
        "import_specifier" | "export_specifier" | "namespace_import" | "import_clause" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "identifier" {
                    out.insert(parsed.node_text(&child).trim().to_string());
                }
            }
        }
        _ => {}
    });
    out
}

/// R2: the root names of every member or subscript chain the owner writes through (an assignment, compound
/// assignment, `++`/`--`, `delete`, a `for` head or a destructuring-assignment member leaf), nested callables
/// included, TypeScript wrappers peeled. A binding twin of an alias spelled like one of them stays: the twin's
/// spurious kill of the target's earlier definitions is, for an alias A (option A, #783) does not certify,
/// what keeps a write through it from leaving a wrong `exact`, and no syntactic proof is made here that A
/// certifies the alias, so the whole class keeps the twin.
pub(super) fn written_through_roots(parsed: &ParsedFile, owner: &Node<'_>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    walk_owner(owner, |node| {
        if !matches!(node.kind(), "member_expression" | "subscript_expression") {
            return;
        }
        let Some(root) = chain_root(parsed, node) else {
            return;
        };
        if is_write_target(node) {
            out.insert(root);
        }
    });
    out
}

/// The identifier a member chain bottoms out in (parentheses and TypeScript wrappers peeled).
fn chain_root(parsed: &ParsedFile, node: Node<'_>) -> Option<String> {
    let mut cur = node;
    loop {
        match cur.kind() {
            "member_expression" | "subscript_expression" => {
                cur = cur.child_by_field_name("object")?
            }
            "parenthesized_expression"
            | "non_null_expression"
            | "as_expression"
            | "satisfies_expression" => cur = cur.named_child(0)?,
            // `<T>x`: the type arguments come first.
            "type_assertion" => cur = cur.named_child(cur.named_child_count().checked_sub(1)?)?,
            "identifier" => return Some(parsed.node_text(&cur).to_string()),
            _ => return None,
        }
    }
}

/// True when `node` (a member or subscript expression) is written: the target of an assignment, an
/// update, a `delete`, a `for` head or a destructuring-assignment leaf. The outermost chain element
/// decides, so an inner element of `u.a.b = 1` is judged through its parent chain.
fn is_write_target(node: Node<'_>) -> bool {
    let mut cur = node;
    while let Some(parent) = cur.parent() {
        match parent.kind() {
            "member_expression" | "subscript_expression" => {
                if parent.child_by_field_name("object").map(|o| o.id()) != Some(cur.id()) {
                    return false;
                }
            }
            "parenthesized_expression"
            | "non_null_expression"
            | "as_expression"
            | "satisfies_expression"
            | "type_assertion" => {}
            "assignment_expression" | "augmented_assignment_expression" => {
                return field_in_parent(cur) == Some("left");
            }
            "update_expression" => return true,
            "unary_expression" => {
                let is_delete = parent
                    .child_by_field_name("operator")
                    .is_some_and(|op| op.kind() == "delete");
                return is_delete;
            }
            "for_in_statement" => return field_in_parent(cur) == Some("left"),
            "pair_pattern" | "array_pattern" | "object_pattern" | "rest_pattern"
            | "spread_element" => {
                return pattern_in_assignment(parent);
            }
            "array" | "object" | "pair" => return pattern_in_assignment(parent),
            _ => return false,
        }
        cur = parent;
    }
    false
}

/// True when the pattern-like node sits (through patterns, arrays and objects) on the left of an assignment
/// or a `for` head.
fn pattern_in_assignment(mut node: Node<'_>) -> bool {
    while let Some(parent) = node.parent() {
        match parent.kind() {
            "pair_pattern"
            | "array_pattern"
            | "object_pattern"
            | "rest_pattern"
            | "spread_element"
            | "array"
            | "object"
            | "pair"
            | "parenthesized_expression" => {}
            "assignment_expression" | "augmented_assignment_expression" => {
                return field_in_parent(node) == Some("left")
            }
            "for_in_statement" => return field_in_parent(node) == Some("left"),
            _ => return false,
        }
        node = parent;
    }
    false
}
