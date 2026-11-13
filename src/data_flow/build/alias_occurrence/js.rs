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
//! below a formal parameter list of the owner (#761: `function f(o, u = o, v = u.x++)` relies on the binding twin
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

const CALLABLES: [&str; 7] = [
    "function_declaration",
    "function_expression",
    "function",
    "generator_function",
    "generator_function_declaration",
    "arrow_function",
    "method_definition",
];

/// True when the node sits below a `formal_parameters` list (or an arrow's single parameter) of `owner`, nested
/// callables inside a default included. With no owner the walk stops at the innermost callable.
pub(super) fn in_parameter_list(node: Node<'_>, owner: Option<&Node<'_>>) -> bool {
    let mut cur = node;
    while let Some(parent) = cur.parent() {
        if parent.kind() == "formal_parameters"
            || (parent.kind() == "arrow_function" && field_in_parent(cur) == Some("parameter"))
        {
            return true;
        }
        match owner {
            Some(o) if parent.id() == o.id() => return false,
            None if CALLABLES.contains(&parent.kind()) => return false,
            _ => {}
        }
        cur = parent;
    }
    false
}

#[cfg(test)]
pub(super) fn classify(parsed: &ParsedFile, span: &PathSpan) -> Classified {
    classify_in(parsed, None, span)
}

pub(super) fn classify_in(
    parsed: &ParsedFile,
    owner: Option<&Node<'_>>,
    span: &PathSpan,
) -> Classified {
    let unrecognised = done(Disposition::Unsupported, "X-UNRECOGNISED");
    let Some(node) = span_node(parsed, span) else {
        return unrecognised;
    };
    if in_parameter_list(node, owner) {
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
            "lexical_declaration" | "variable_declaration" => {
                return !in_parameter_list(node, None)
            }
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

/// R2: the root names of every member or subscript path the owner writes through, taken from the write census
/// itself (`js_write_census`: its target recognition, wrappers, pattern kinds and root kinds), so a form the census
/// records as a direct write of a path is a write through the alias here. A plain rebinding is no member write; a
/// default's `right` is a read. `None` when the census refuses the owner (parse error, escaped name, refused kind):
/// the caller then keeps every twin. A binding twin of an alias whose target some written root resolves to stays: the
/// twin's spurious kill of the target's earlier defs is what keeps a write through an alias that option A (#783) does
/// not certify off `exact`, and no proof is made here that A certifies the alias.
pub(super) fn written_through_roots(
    parsed: &ParsedFile,
    owner: &Node<'_>,
) -> Option<BTreeSet<String>> {
    let census = crate::js_write_census::walk::census_with(
        parsed,
        crate::js_write_census::tables::Tables::shared(),
        *owner,
        false,
    );
    if census.refused() {
        return None;
    }
    Some(
        census
            .sites
            .iter()
            .filter(|s| {
                s.form != crate::js_write_census::walk::Form::Declare && !s.path.comps.is_empty()
            })
            .map(|s| s.path.base.clone())
            .collect(),
    )
}
